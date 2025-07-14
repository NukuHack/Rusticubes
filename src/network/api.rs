use crate::network::{discovery, types::{NetworkSystem, HostInfo, NetworkStatus, NetworkEvent}};
use ggrs::SessionState;
use std::sync::atomic::{AtomicPtr, AtomicBool, Ordering};
use std::{time::{Duration, Instant}, ptr};

static NETWORK_SYSTEM_PTR: AtomicPtr<NetworkSystem> = AtomicPtr::new(ptr::null_mut());
static NETWORK_INITIALIZED: AtomicBool = AtomicBool::new(false);

#[inline] pub fn get_ptr() -> Option<&'static mut NetworkSystem> {
    let ptr = NETWORK_SYSTEM_PTR.load(Ordering::Acquire);
    if ptr.is_null() { None } else { unsafe { Some(&mut *ptr) } }
}

impl NetworkSystem {
    pub fn update_game_session(&mut self) {
        if let Some(mut session) = self.session.take() {
            session.poll_remote_clients();
            let mut events = Vec::new();
            let mut should_return = false;

            match session.current_state() {
                SessionState::Running => {
                    if matches!(self.status, NetworkStatus::Connected) {
                        self.status = NetworkStatus::InGame;
                        events.push(NetworkEvent::Ready);
                    }
                    
                    let input = (self.frame_count % 15) as i32;
                    if let Err(e) = session.add_local_input(self.local_player_id, input) {
                        self.consecutive_errors += 1;
                        if self.consecutive_errors > 10 {
                            events.push(NetworkEvent::Error(format!("Input error: {:?}", e)));
                            should_return = true;
                        }
                    }

                    if !should_return {
                        match session.advance_frame() {
                            Ok(_) => {
                                self.consecutive_errors = 0;
                                self.game_state += 1;
                                self.frame_count += 1;
                                if self.frame_count % 60 == 0 {
                                    events.push(NetworkEvent::GameStateUpdate(self.game_state));
                                }
                            }
                            Err(e) => {
                                self.consecutive_errors += 1;
                                match e {
                                    ggrs::GgrsError::PredictionThreshold if self.consecutive_errors > 12 => {
                                        events.push(NetworkEvent::Error("Prediction threshold".to_string()));
                                    }
                                    ggrs::GgrsError::NotSynchronized => {
                                        events.push(NetworkEvent::Synchronizing);
                                    }
                                    _ => events.push(NetworkEvent::Error(format!("Frame error: {:?}", e))),
                                }
                            }
                        }
                    }
                }
                SessionState::Synchronizing => {
                    events.push(NetworkEvent::Synchronizing);
                    if let Some(start) = self.sync_start_time {
                        if start.elapsed() > Duration::from_secs(15) {
                            events.push(NetworkEvent::Error("Sync timeout".to_string()));
                        }
                    }
                }
            }

            self.session = Some(session);
            for event in events { self.push_event(event); }
        }
    }
}

#[inline] pub fn init_network(is_host: bool) -> Result<String, String> {
    if NETWORK_INITIALIZED.load(Ordering::Acquire) {
        return Err("Already initialized".to_string());
    }
    
    let old_ptr = NETWORK_SYSTEM_PTR.swap(Box::into_raw(Box::new(NetworkSystem::new(is_host))), Ordering::AcqRel);
    if !old_ptr.is_null() { unsafe { drop(Box::from_raw(old_ptr)); } }
    
    NETWORK_INITIALIZED.store(true, Ordering::Release);
    Ok(format!("Initialized as {}", if is_host { "host" } else { "client" }))
}

#[inline] pub fn is_running() -> bool { NETWORK_INITIALIZED.load(Ordering::Acquire) }
#[inline] pub fn is_host() -> Result<bool, String> {
    get_ptr().map(|s| s.is_host).ok_or("Not initialized".to_string())
}

#[inline] pub fn begin_online_search() -> Result<String, String> {
    cleanup_network();
    init_network(false).and_then(|init_msg| {
        discovery::discover_hosts(200)
            .map(|discovery_msg| format!("{} | {}", init_msg, discovery_msg))
            .map_err(|e| format!("{} | Error: {}", init_msg, e))
    })
}

#[inline] pub fn begin_online_giveaway() -> Result<String, String> {
    cleanup_network();
    init_network(true).and_then(|init_msg| {
        
    get_ptr().map_or(Err("Not initialized".to_string()), |s| {
        s.setup_tcp_listener().map(|msg| {
            s.status = NetworkStatus::Discovering;
            format!("TCP: {} | Discovering", msg)
        })
    }).and_then(|host_msg| {
            if let Some(s) = get_ptr() { let _ = s.start_broadcast_listener(); }
            Ok(format!("{} | {}", init_msg, host_msg))
        }).map_err(|e| format!("{} | {}", init_msg, e))
    })
}

#[inline] pub fn cleanup_network() {
    if let Some(s) = get_ptr() {
        if let Some(h) = s.discovery_thread.take() { let _ = h.join(); }
        s.broadcast_listener_thread.take();
    }
    
    let old_ptr = NETWORK_SYSTEM_PTR.swap(ptr::null_mut(), Ordering::AcqRel);
    if !old_ptr.is_null() { unsafe { drop(Box::from_raw(old_ptr)); } }
    NETWORK_INITIALIZED.store(false, Ordering::Release);
}

#[inline] pub fn update_network() {
    handle_network_events();
    
    if let Some(s) = get_ptr() {
        s.check_discovery_complete();
        
        if let Ok(true) = s.check_pending_connections() {
            if let Err(e) = s.setup_ggrs_session() {
                s.status = NetworkStatus::Error(e.clone());
                s.push_event(NetworkEvent::Error(e));
            }
        }
        
        let now = Instant::now();
        if now.duration_since(s.last_frame_time) < Duration::from_millis(16) { return; }
        s.last_frame_time = now;
        
        match s.status {
            NetworkStatus::Discovering if s.is_host => {
                if let Err(e) = s.try_accept_connection() {
                    // Don't treat "No connection" as an error
                    if !e.contains("No connection") {
                        let msg = format!("Host error: {}", e);
                        s.status = NetworkStatus::Error(msg.clone());
                        s.push_event(NetworkEvent::Error(msg));
                    }
                }
            }
            NetworkStatus::Connecting if !s.is_host => {
            // Only try to connect if we haven't already started
                if s.pending_connections.is_empty() {
                    let target_ip = s.target_host_ip.clone();
                    if let Some(ip) = target_ip {
                        if let Err(e) = s.try_connect_to_host(&ip) {
                            let msg = format!("Connection error: {}", e);
                            s.status = NetworkStatus::Error(msg.clone());
                            s.push_event(NetworkEvent::Error(msg));
                        }
                    } else {
                        let msg = "No target IP".to_string();
                        s.status = NetworkStatus::Error(msg.clone());
                        s.push_event(NetworkEvent::Error(msg));
                    }
                }
            }
            NetworkStatus::Connected | NetworkStatus::InGame => s.update_game_session(),
            _ => {}
        }
    }
}

fn handle_network_events() {
    while let Some(event) = pop_event() {
        match event {
            NetworkEvent::DiscoveryComplete(res) => {
                println!("Found {} hosts", res.hosts.len());

                process_discovered_hosts(res.hosts);

                for info in &res.debug_info { println!("Debug: {}", info); }
                for err in &res.errors { println!("Error: {}", err); }
            }
            NetworkEvent::Connected(addr) => println!("Connected to {}", addr),
            NetworkEvent::Ready => println!("Game ready!"),
            NetworkEvent::GameStateUpdate(s) => println!("State: {}", s),
            NetworkEvent::Error(e) => println!("Error: {}", e),
            NetworkEvent::Synchronizing => println!("Syncing..."),
            _ => {}
        }
    }
}

pub fn process_discovered_hosts(hosts: Vec<HostInfo>) {
    let mut debug:Vec<String> = Vec::new();
    let mut error:Vec<String> = Vec::new();

    for (i, host) in hosts.iter().enumerate() {
        println!("Host {}. add: {} , pid: {} ({})", i, host.address, host.pid, host.world_name);

        let name = NetworkSystem::get_world_name_from_host(host.address, &mut debug, &mut error).unwrap_or("Unknown".to_string());

        println!("World name: {} and checked to be: {}", host.world_name, name);
    }
    
    for info in &debug { println!("Debug: {}", info); }
    for err in &error { println!("Error: {}", err); }
}

#[inline] pub fn get_discovered_hosts() -> Vec<HostInfo> {
    get_ptr().map_or(Vec::new(), |s| s.discovered_hosts.lock().unwrap().clone())
}

#[inline] pub fn connect_to_host(ip: &str) -> Result<String, String> {
    get_ptr().map_or(Err("Not initialized".to_string()), |s| {
        s.status = NetworkStatus::Connecting;
        s.set_target_host_ip(ip.to_string());
        Ok(format!("Connecting to {}", ip))
    })
}

#[inline] pub fn get_status() -> NetworkStatus {
    get_ptr().map_or_else(
        || NetworkStatus::Error("Not initialized".to_string()),
        |s| s.status.clone()
    )
}

#[inline] pub fn pop_event() -> Option<NetworkEvent> {
    get_ptr().and_then(|s| s.event_queue.pop_front())
}

#[inline] pub fn refresh_discovery() -> Result<String, String> {
    get_ptr().map_or(Err("Not initialized".to_string()), |s| {
        if s.is_host { return Err("Cannot refresh as host".to_string()); }
        if s.discovery_thread.is_some() { return Ok("Already discovering".to_string()); }
        discovery::discover_hosts(200)
    })
}
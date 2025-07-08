
use crate::network::discovery;
use crate::network::types::{NetworkSystem, HostInfo, NetworkStatus, NetworkEvent};
use ggrs::SessionState;
use std::sync::atomic::{AtomicPtr, AtomicBool, Ordering};
use std::time::{Duration, Instant};
use std::ptr;

static NETWORK_SYSTEM_PTR: AtomicPtr<NetworkSystem> = AtomicPtr::new(ptr::null_mut());
static NETWORK_INITIALIZED: AtomicBool = AtomicBool::new(false);

// Helper function to safely access the NetworkSystem pointer
#[inline]
pub fn get_ptr() -> Option<&'static mut NetworkSystem> {
    let system_ptr = NETWORK_SYSTEM_PTR.load(Ordering::Acquire);
    if system_ptr.is_null() {
        None
    } else {
        unsafe { Some(&mut *system_ptr) }
    }
}

impl NetworkSystem {
    pub fn update_game_session(&mut self) {
        let session_option = self.session.take();
        if let Some(mut session) = session_option {
            session.poll_remote_clients();
            
            let session_state = session.current_state();
            let local_player_id = self.local_player_id;
            let frame_count = self.frame_count;
            
            // Collect events to push later
            let mut events_to_push = Vec::new();
            let mut should_return = false;
            
            match session_state {
                SessionState::Running => {
                    // Check if we need to update status
                    let should_update_status = matches!(self.status, NetworkStatus::Connected);
                    if should_update_status {
                        self.status = NetworkStatus::InGame;
                        events_to_push.push(NetworkEvent::Ready);
                    }
                    
                    // Generate input
                    let input = (frame_count % 5) as i32;
                    
                    // Add local input
                    if let Err(e) = session.add_local_input(local_player_id, input) {
                        self.consecutive_errors += 1;
                        if self.consecutive_errors > 10 {
                            events_to_push.push(NetworkEvent::Error(format!("Too many input errors: {:?}", e)));
                            should_return = true;
                        }
                    }
                    
                    if !should_return {
                        // Advance frame
                        match session.advance_frame() {
                            Ok(_) => {
                                self.consecutive_errors = 0;
                                self.game_state += 1;
                                self.frame_count += 1;
                                
                                // Push periodic state updates
                                if self.frame_count % 60 == 0 {
                                    events_to_push.push(NetworkEvent::GameStateUpdate(self.game_state));
                                }
                            }
                            Err(e) => {
                                self.consecutive_errors += 1;
                                match e {
                                    ggrs::GgrsError::PredictionThreshold => {
                                        if self.consecutive_errors > 10 {
                                            events_to_push.push(NetworkEvent::Error("Prediction threshold exceeded".to_string()));
                                        }
                                    }
                                    ggrs::GgrsError::NotSynchronized => {
                                        events_to_push.push(NetworkEvent::Synchronizing);
                                    }
                                    _ => {
                                        events_to_push.push(NetworkEvent::Error(format!("Frame advance error: {:?}", e)));
                                    }
                                }
                            }
                        }
                    }
                }
                SessionState::Synchronizing => {
                    events_to_push.push(NetworkEvent::Synchronizing);
                    
                    // Check sync timeout
                    if let Some(sync_start) = self.sync_start_time {
                        if sync_start.elapsed() > Duration::from_secs(15) {
                            events_to_push.push(NetworkEvent::Error("Sync timeout".to_string()));
                        }
                    }
                }
            }
            
            // Put the session back
            self.session = Some(session);
            
            // Push all collected events
            for event in events_to_push {
                self.push_event(event);
            }
        }
    }
}

/// Initialize the network system
#[inline]
pub fn init_network(is_host: bool) -> Result<String, String> {
    if NETWORK_INITIALIZED.load(Ordering::Acquire) {
        return Err("Network system already initialized".to_string());
    }
    
    let system = Box::new(NetworkSystem::new(is_host));
    
    let old_ptr = NETWORK_SYSTEM_PTR.swap(Box::into_raw(system), Ordering::AcqRel);
    if !old_ptr.is_null() {
        unsafe { let _ = Box::from_raw(old_ptr); }
    }
    
    NETWORK_INITIALIZED.store(true, Ordering::Release);
    
    Ok(format!("Network system initialized as {}", if is_host { "host" } else { "client" }))
}

#[inline]
pub fn is_running() -> bool {
    NETWORK_INITIALIZED.load(Ordering::Acquire)
}

#[inline]
pub fn is_host() -> Result<bool, String>  {
    if let Some(system) = get_ptr() {
        Ok(system.is_host)
    } else {
        Err("Network system not initialized".to_string())
    }
}


#[inline]
pub fn begin_online_search() -> Result<String, String> {
    // Simple approach - always restart as client
    cleanup_network();
    match init_network(false) {
        Ok(init_msg) => {
            match discovery::discover_hosts(100) {
                Ok(discovery_msg) => Ok(format!(" Init: {} | Discovery: {}", init_msg, discovery_msg)),
                Err(e) => Err(format!(" Init: {} | Discovery error: {}", init_msg, e)),
            }
        },
        Err(e) => Err(format!(" Init error: {}", e)),
    }
}

#[inline]
pub fn begin_online_giveaway() -> Result<String, String> {
    // Simple approach - always restart as host
    cleanup_network();
    match init_network(true) {
        Ok(init_msg) => {
            match start_host() {
                Ok(host_msg) => Ok(format!(" Init: {} | Host: {}", init_msg, host_msg)),
                Err(e) => Err(format!(" Init: {} | Host error: {}", init_msg, e)),
            }
        },
        Err(e) => Err(format!(" Init error: {}", e)),
    }
}


/// Start hosting a game
#[inline]
pub fn start_host() -> Result<String, String> {
    if let Some(system) = get_ptr() {
        match system.setup_tcp_listener() {
            Ok(listener_msg) => {
                system.status = NetworkStatus::Discovering;
                Ok(format!("TCP setup: {} | Status set to Discovering", listener_msg))
            }
            Err(e) => Err(format!("TCP setup failed: {}", e))
        }
    } else {
        Err("Network system not initialized".to_string())
    }
}


/// Cleanup the network system
#[inline]
pub fn cleanup_network() {
    // Wait for discovery thread to finish if it's running
    if let Some(system) = get_ptr() {
        if let Some(handle) = system.discovery_thread.take() {
            let _ = handle.join();
        }
    }
    
    let old_ptr = NETWORK_SYSTEM_PTR.swap(ptr::null_mut(), Ordering::AcqRel);
    if !old_ptr.is_null() {
        unsafe { let _ = Box::from_raw(old_ptr); }
    }
    NETWORK_INITIALIZED.store(false, Ordering::Release);
}


/// Update the network system - call this every frame
#[inline]
pub fn update_network() {
    if let Some(system) = get_ptr() {
        // Check if discovery is complete
        system.check_discovery_complete();
        
        // Limit update rate to ~60 FPS
        let now = Instant::now();
        if now.duration_since(system.last_frame_time) < Duration::from_millis(16) {
            return;
        }
        system.last_frame_time = now;
        
        match &system.status {
            NetworkStatus::Discovering => {
                if system.is_host {
                    match system.try_accept_connection() {
                        Ok((true, _)) => {
                            if let Err(e) = system.setup_ggrs_session() {
                                system.status = NetworkStatus::Error(e.clone());
                                system.push_event(NetworkEvent::Error(e));
                            }
                        }
                        Ok((false, _)) => {
                            // No connection yet, keep waiting
                        }
                        Err(e) => {
                            let error_msg = format!("Host connection error: {}", e);
                            system.status = NetworkStatus::Error(error_msg.clone());
                            system.push_event(NetworkEvent::Error(error_msg));
                        }
                    }
                }
            }
            NetworkStatus::Connecting => {
                if !system.is_host {
                    match system.try_connect_to_host(0) { // target_pid not used in current implementation
                        Ok((true,_)) => {
                            if let Err(e) = system.setup_ggrs_session() {
                                system.status = NetworkStatus::Error(e.clone());
                                system.push_event(NetworkEvent::Error(e));
                            }
                        }
                        Ok((false,_)) => {
                            // Connection in progress
                        }
                        Err(e) => {
                            let error_msg = format!("Connection error: {}", e);
                            system.status = NetworkStatus::Error(error_msg.clone());
                            system.push_event(NetworkEvent::Error(error_msg));
                        }
                    }
                }
            }
            NetworkStatus::Connected | NetworkStatus::InGame => {
                system.update_game_session();
            }
            _ => {}
        }
    }
}


/// Get the list of discovered hosts
#[inline]
pub fn get_discovered_hosts() -> Vec<HostInfo> {
    if let Some(system) = get_ptr() {
        system.discovered_hosts.lock().unwrap().clone()
    } else {
        Vec::new()
    }
}


/// Try to connect to a host
#[inline]
pub fn connect_to_host(_target_pid: u32) -> Result<String, String> {
    if let Some(system) = get_ptr() {
        system.status = NetworkStatus::Connecting;
        Ok(format!("Status set to Connecting for target PID: {}", _target_pid))
    } else {
        Err("Network system not initialized".to_string())
    }
}

/// Get the current network status
#[inline]
pub fn get_status() -> NetworkStatus {
    if let Some(system) = get_ptr() {
        system.status.clone()
    } else {
        NetworkStatus::Error("Network system not initialized".to_string())
    }
}

/// Get the next network event from the queue
#[inline]
pub fn pop_event() -> Option<NetworkEvent> {
    if let Some(system) = get_ptr() {
        system.event_queue.pop_front()
    } else {
        None
    }
}

/// Get current game state
#[inline]
pub fn get_game_state() -> i32 {
    if let Some(system) = get_ptr() {
        system.game_state
    } else {
        0
    }
}

/// Get current frame count
#[inline]
pub fn get_frame_count() -> u32 {
    if let Some(system) = get_ptr() {
        system.frame_count
    } else {
        0
    }
}


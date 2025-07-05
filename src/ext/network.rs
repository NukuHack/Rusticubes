use crate::config;
use std::io::{BufRead, BufReader, Write};
use ggrs::{Config, PlayerType, SessionBuilder, UdpNonBlockingSocket, SessionState};
use serde::{Deserialize, Serialize};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicPtr, AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::ptr;
use std::collections::VecDeque;
use std::io;

#[derive(Debug, Serialize, Deserialize)]
enum NetworkMessage {
    PeerAddress(SocketAddr),
    Ping,
    Pong,
    JoinRequest(u32),
    JoinResponse(SocketAddr),
    WorldInfoRequest,
    WorldInfoResponse(String), // Contains world name
}

#[derive(Debug)]
struct GameConfig;

impl Config for GameConfig {
    type Input = i32;
    type State = i32;
    type Address = SocketAddr;
}

#[derive(Debug, Clone)]
pub struct DiscoveryResult {
    pub hosts: Vec<HostInfo>,
    pub debug_info: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum NetworkEvent {
    Connected(SocketAddr),
    Disconnected,
    GameStateUpdate(i32),
    Error(String),
    Synchronizing,
    Ready,
    HostsDiscovered(Vec<(u32, SocketAddr, String)>), // pid, address, world_name
    DiscoveryComplete(DiscoveryResult),
}

#[derive(Debug, Clone)]
pub enum NetworkStatus {
    Idle,
    Discovering,
    Connecting,
    Connected,
    InGame,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct HostInfo {
    pub pid: u32,
    pub address: SocketAddr,
    pub world_name: String,
}

struct NetworkSystem {
    status: NetworkStatus,
    is_host: bool,
    local_player_id: usize,
    session: Option<ggrs::P2PSession<GameConfig>>,
    tcp_listener: Option<TcpListener>,
    local_udp_addr: Option<SocketAddr>,
    remote_udp_addr: Option<SocketAddr>,
    frame_count: u32,
    game_state: i32,
    last_frame_time: Instant,
    event_queue: VecDeque<NetworkEvent>,
    consecutive_errors: u32,
    sync_start_time: Option<Instant>,
    current_pid: u32,
    discovery_thread: Option<thread::JoinHandle<DiscoveryResult>>,
    discovered_hosts: Arc<Mutex<Vec<HostInfo>>>,
}

static NETWORK_SYSTEM_PTR: AtomicPtr<NetworkSystem> = AtomicPtr::new(ptr::null_mut());
static NETWORK_INITIALIZED: AtomicBool = AtomicBool::new(false);

// Helper function to safely access the NetworkSystem pointer
#[inline]
fn get_ptr() -> Option<&'static mut NetworkSystem> {
    let system_ptr = NETWORK_SYSTEM_PTR.load(Ordering::Acquire);
    if system_ptr.is_null() {
        None
    } else {
        unsafe { Some(&mut *system_ptr) }
    }
}

impl NetworkSystem {
    fn new(is_host: bool) -> Self {
        let current_pid = std::process::id();
        Self {
            status: NetworkStatus::Idle,
            is_host,
            local_player_id: if is_host { 0 } else { 1 },
            session: None,
            tcp_listener: None,
            local_udp_addr: None,
            remote_udp_addr: None,
            frame_count: 0,
            game_state: 0,
            last_frame_time: Instant::now(),
            event_queue: VecDeque::new(),
            consecutive_errors: 0,
            sync_start_time: None,
            current_pid,
            discovery_thread: None,
            discovered_hosts: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn push_event(&mut self, event: NetworkEvent) {
        self.event_queue.push_back(event);
        // Keep queue size reasonable
        if self.event_queue.len() > 100 {
            self.event_queue.pop_front();
        }
    }

    fn setup_tcp_listener(&mut self) -> Result<String, String> {
        if self.is_host && self.tcp_listener.is_none() {
            let port = 9000;
            let addr = format!("127.0.0.1:{}", port);
            match TcpListener::bind(&addr) {
                Ok(listener) => {
                    match listener.set_nonblocking(true) {
                        Ok(_) => {
                            self.tcp_listener = Some(listener);
                            Ok(format!("Successfully bound TCP listener to {}", addr))
                        }
                        Err(e) => Err(format!("Failed to set TCP listener non-blocking: {}", e))
                    }
                }
                Err(e) => Err(format!("Failed to bind TCP listener to {}: {}", addr, e))
            }
        } else if self.is_host {
            Ok("TCP listener already exists".to_string())
        } else {
            Ok("Not host, no TCP listener needed".to_string())
        }
    }

    fn handle_client_message(&mut self, stream: &mut TcpStream, message: NetworkMessage) -> Result<(bool, String), String> {
        match message {
            NetworkMessage::WorldInfoRequest => {
                let world_name = config::get_gamestate().worldname().to_string();
                let response = NetworkMessage::WorldInfoResponse(world_name.clone());
                match serde_json::to_string(&response) {
                    Ok(response_json) => {
                        match writeln!(stream, "{}", response_json) {
                            Ok(_) => Ok((false, format!("Sent world info response: {}", world_name))),
                            Err(e) => Err(format!("Failed to write world info response: {}", e))
                        }
                    }
                    Err(e) => Err(format!("Failed to serialize world info response: {}", e))
                }
            }
            NetworkMessage::JoinRequest(peer_pid) => {
                let debug_msg = format!("Received join request from peer PID: {}", peer_pid);
                
                // Setup UDP socket
                let udp_port = 7000 + (self.current_pid % 1000) as u16;
                let udp_addr = SocketAddr::new(
                    std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
                    udp_port,
                );
                
                // Send response
                let response = NetworkMessage::JoinResponse(udp_addr);
                match serde_json::to_string(&response) {
                    Ok(response_json) => {
                        match writeln!(stream, "{}", response_json) {
                            Ok(_) => {
                                // Read peer's UDP address
                                let mut reader = BufReader::new(stream);
                                let mut peer_line = String::new();
                                match reader.read_line(&mut peer_line) {
                                    Ok(_) => {
                                        match serde_json::from_str::<NetworkMessage>(&peer_line.trim()) {
                                            Ok(peer_msg) => {
                                                if let NetworkMessage::PeerAddress(peer_udp_addr) = peer_msg {
                                                    self.local_udp_addr = Some(udp_addr);
                                                    self.remote_udp_addr = Some(peer_udp_addr);
                                                    Ok((true, format!("{} - Successfully exchanged UDP addresses", debug_msg)))
                                                } else {
                                                    Err(format!("{} - Received wrong message type from peer", debug_msg))
                                                }
                                            }
                                            Err(e) => Err(format!("{} - Failed to parse peer message: {}", debug_msg, e))
                                        }
                                    }
                                    Err(e) => Err(format!("{} - Failed to read peer UDP address: {}", debug_msg, e))
                                }
                            }
                            Err(e) => Err(format!("{} - Failed to write join response: {}", debug_msg, e))
                        }
                    }
                    Err(e) => Err(format!("{} - Failed to serialize join response: {}", debug_msg, e))
                }
            }
            _ => Ok((false, "Received other message type".to_string()))
        }
    }

    fn try_accept_connection(&mut self) -> Result<(bool, String), String> {
        if let Some(ref listener) = self.tcp_listener {
            match listener.accept() {
                Ok((mut stream, addr)) => {
                    let debug_msg = format!("Accepted connection from {}", addr);
                    match stream.set_nonblocking(true) {
                        Ok(_) => {
                            // Try to read message
                            let mut reader = BufReader::new(&stream);
                            let mut line = String::new();
                            
                            match reader.read_line(&mut line) {
                                Ok(0) => Ok((false, format!("{} - No data received yet", debug_msg))),
                                Ok(_) => {
                                    match serde_json::from_str::<NetworkMessage>(&line.trim()) {
                                        Ok(msg) => {
                                            drop(reader); // Drop reader to use stream mutably
                                            match self.handle_client_message(&mut stream, msg) {
                                                Ok((should_connect, msg_debug)) => {
                                                    Ok((should_connect, format!("{} - {}", debug_msg, msg_debug)))
                                                }
                                                Err(e) => Err(format!("{} - Message handling error: {}", debug_msg, e))
                                            }
                                        }
                                        Err(e) => Err(format!("{} - Failed to parse message: {}", debug_msg, e))
                                    }
                                }
                                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                                    Ok((false, format!("{} - No data available yet", debug_msg)))
                                }
                                Err(e) => Err(format!("{} - Read error: {}", debug_msg, e))
                            }
                        }
                        Err(e) => Err(format!("{} - Failed to set stream non-blocking: {}", debug_msg, e))
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    Ok((false, "No connection available".to_string()))
                }
                Err(e) => Err(format!("Accept error: {}", e))
            }
        } else {
            Ok((false, "No TCP listener available".to_string()))
        }
    }

    fn try_connect_to_host(&mut self, _target_pid: u32) -> Result<(bool, String), String> {
        let host_addr = "127.0.0.1:9000";
        
        match TcpStream::connect(host_addr) {
            Ok(mut stream) => {
                let debug_msg = format!("Connected to host at {}", host_addr);
                match stream.set_nonblocking(true) {
                    Ok(_) => {
                        // Setup UDP socket
                        let udp_port = 7000 + (self.current_pid % 1000) as u16;
                        let local_udp_addr = SocketAddr::new(
                            std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
                            udp_port,
                        );
                        
                        // Send join request
                        let join_msg = NetworkMessage::JoinRequest(self.current_pid);
                        match serde_json::to_string(&join_msg) {
                            Ok(join_json) => {
                                match writeln!(stream, "{}", join_json) {
                                    Ok(_) => {
                                        // Read response
                                        let mut reader = BufReader::new(&stream);
                                        let mut line = String::new();
                                        match reader.read_line(&mut line) {
                                            Ok(_) => {
                                                match serde_json::from_str::<NetworkMessage>(&line.trim()) {
                                                    Ok(msg) => {
                                                        if let NetworkMessage::JoinResponse(host_udp_addr) = msg {
                                                            // Send our UDP address
                                                            let peer_msg = NetworkMessage::PeerAddress(local_udp_addr);
                                                            match serde_json::to_string(&peer_msg) {
                                                                Ok(peer_json) => {
                                                                    drop(reader); // Drop reader to use stream mutably
                                                                    match writeln!(stream, "{}", peer_json) {
                                                                        Ok(_) => {
                                                                            self.local_udp_addr = Some(local_udp_addr);
                                                                            self.remote_udp_addr = Some(host_udp_addr);
                                                                            Ok((true, format!("{} - Successfully exchanged UDP addresses", debug_msg)))
                                                                        }
                                                                        Err(e) => Err(format!("{} - Failed to send UDP address: {}", debug_msg, e))
                                                                    }
                                                                }
                                                                Err(e) => Err(format!("{} - Failed to serialize peer address: {}", debug_msg, e))
                                                            }
                                                        } else {
                                                            Err(format!("{} - Received wrong response type", debug_msg))
                                                        }
                                                    }
                                                    Err(e) => Err(format!("{} - Failed to parse host response: {}", debug_msg, e))
                                                }
                                            }
                                            Err(e) => Err(format!("{} - Failed to read host response: {}", debug_msg, e))
                                        }
                                    }
                                    Err(e) => Err(format!("{} - Failed to send join request: {}", debug_msg, e))
                                }
                            }
                            Err(e) => Err(format!("{} - Failed to serialize join request: {}", debug_msg, e))
                        }
                    }
                    Err(e) => Err(format!("{} - Failed to set stream non-blocking: {}", debug_msg, e))
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                Ok((false, "Connection in progress".to_string()))
            }
            Err(e) => Err(format!("Failed to connect to {}: {}", host_addr, e))
        }
    }

    fn setup_ggrs_session(&mut self) -> Result<String, String> {
        if let (Some(local_addr), Some(remote_addr)) = (self.local_udp_addr, self.remote_udp_addr) {
            let debug_msg = format!("Setting up GGRS session - Local: {}, Remote: {}", local_addr, remote_addr);
            
            match UdpNonBlockingSocket::bind_to_port(local_addr.port()) {
                Ok(udp_socket) => {
                    let remote_player_id = if self.is_host { 1 } else { 0 };
                    
                    match SessionBuilder::<GameConfig>::new()
                        .with_num_players(2)
                        .with_input_delay(2)
                        .with_fps(60)
                        .unwrap()
                        .with_max_prediction_window(6)
                        .expect("Failed to set prediction window")
                        .with_check_distance(30)
                        .add_player(PlayerType::Local, self.local_player_id)
                        .unwrap()
                        .add_player(PlayerType::Remote(remote_addr), remote_player_id)
                        .unwrap()
                        .start_p2p_session(udp_socket)
                    {
                        Ok(session) => {
                            self.session = Some(session);
                            self.sync_start_time = Some(Instant::now());
                            self.status = NetworkStatus::Connected;
                            self.push_event(NetworkEvent::Connected(remote_addr));
                            Ok(format!("{} - GGRS session created successfully", debug_msg))
                        }
                        Err(e) => Err(format!("{} - Failed to create GGRS session: {:?}", debug_msg, e))
                    }
                }
                Err(e) => Err(format!("{} - Failed to bind UDP socket: {:?}", debug_msg, e))
            }
        } else {
            Err(format!("UDP addresses not set - Local: {:?}, Remote: {:?}", self.local_udp_addr, self.remote_udp_addr))
        }
    }

    fn update_game_session(&mut self) {
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

    fn start_discovery_thread(&mut self, timeout_ms: u64) {
        let handle = thread::spawn(move || {
            discover_hosts_threaded(timeout_ms)
        });
        
        self.discovery_thread = Some(handle);
    }

    fn check_discovery_complete(&mut self) -> bool {
        if let Some(handle) = self.discovery_thread.take() {
            if handle.is_finished() {
                match handle.join() {
                    Ok(result) => {
                        self.push_event(NetworkEvent::DiscoveryComplete(result));
                        return true;
                    }
                    Err(_) => {
                        self.push_event(NetworkEvent::Error("Discovery thread panicked".to_string()));
                        return true;
                    }
                }
            } else {
                // Put the handle back
                self.discovery_thread = Some(handle);
            }
        }
        false
    }
}

fn discover_hosts_threaded(timeout_ms: u64) -> DiscoveryResult {
    use std::net::{IpAddr, Ipv4Addr};
    
    let mut hosts = Vec::new();
    let mut debug_info = Vec::new();
    let mut errors = Vec::new();
    
    debug_info.push("Starting host discovery".to_string());
    
    // Scan for potential hosts
    for port in 9000..=9009 {
        let addr = SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port,
        );
        
        debug_info.push(format!("Attempting to connect to {}", addr));
        
        match TcpStream::connect_timeout(&addr, Duration::from_millis(timeout_ms)) {
            Ok(mut stream) => {
                debug_info.push(format!("Successfully connected to {}", addr));
                
                // Request world info
                let world_info_msg = NetworkMessage::WorldInfoRequest;
                match serde_json::to_string(&world_info_msg) {
                    Ok(world_info_json) => {
                        debug_info.push(format!("Sending world info request to {}", addr));
                        
                        match writeln!(stream, "{}", world_info_json) {
                            Ok(_) => {
                                debug_info.push(format!("Successfully sent world info request to {}", addr));
                                
                                let mut reader = BufReader::new(&stream);
                                let mut line = String::new();
                                match reader.read_line(&mut line) {
                                    Ok(bytes_read) => {
                                        debug_info.push(format!("Read {} bytes from {}: '{}'", bytes_read, addr, line.trim()));
                                        
                                        match serde_json::from_str::<NetworkMessage>(&line.trim()) {
                                            Ok(NetworkMessage::WorldInfoResponse(world_name)) => {
                                                let pid = port as u32;
                                                debug_info.push(format!("Host found - PID: {}, Address: {}, World: {}", pid, addr, world_name));
                                                hosts.push(HostInfo {
                                                    pid,
                                                    address: addr,
                                                    world_name,
                                                });
                                            }
                                            Ok(other_msg) => {
                                                errors.push(format!("Received unexpected message type from {}: {:?}", addr, other_msg));
                                            }
                                            Err(e) => {
                                                errors.push(format!("Failed to parse response from {}: {} - Raw: '{}'", addr, e, line.trim()));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        errors.push(format!("Failed to read response from {}: {}", addr, e));
                                    }
                                }
                            }
                            Err(e) => {
                                errors.push(format!("Failed to send world info request to {}: {}", addr, e));
                            }
                        }
                    }
                    Err(e) => {
                        errors.push(format!("Failed to serialize world info request for {}: {}", addr, e));
                    }
                }
            }
            Err(e) => {
                // Don't log connection refused as errors since that's expected for ports without hosts
                if e.kind() != io::ErrorKind::ConnectionRefused {
                    errors.push(format!("Failed to connect to {}: {}", addr, e));
                } else {
                    debug_info.push(format!("No host at {}: {}", addr, e));
                }
            }
        }
    }
    
    debug_info.push(format!("Discovery complete. Found {} hosts", hosts.len()));
    
    DiscoveryResult {
        hosts,
        debug_info,
        errors,
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
            match discover_hosts(100) {
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

/// Start discovering hosts in a background thread
#[inline]
pub fn discover_hosts(timeout_ms: u64) -> Result<String, String> {
    if let Some(system) = get_ptr() {
        system.start_discovery_thread(timeout_ms);
        Ok(format!("Discovery thread started with timeout: {}ms", timeout_ms))
    } else {
        Err("Network system not initialized".to_string())
    }
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

/// Get the current network status
#[inline]
pub fn get_network_status() -> NetworkStatus {
    if let Some(system) = get_ptr() {
        system.status.clone()
    } else {
        NetworkStatus::Error("Network system not initialized".to_string())
    }
}

/// Get the next network event from the queue
#[inline]
pub fn pop_network_event() -> Option<NetworkEvent> {
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

/// Get the list of discovered hosts
#[inline]
pub fn get_discovered_hosts() -> Vec<HostInfo> {
    if let Some(system) = get_ptr() {
        system.discovered_hosts.lock().unwrap().clone()
    } else {
        Vec::new()
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

use ggrs::{Config, SessionBuilder, UdpNonBlockingSocket, PlayerType};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::process::Command;
use std::time::Instant;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread;
use std::io;

#[derive(Debug, Serialize, Deserialize)]
pub enum NetworkMessage {
    PeerAddress(SocketAddr),
    Ping,
    Pong,
    JoinRequest(u32),
    JoinResponse(SocketAddr),
    WorldInfoRequest,
    WorldInfoResponse(String), // Contains world name
}

#[derive(Debug)]
pub struct HostConfig;

impl Config for HostConfig {
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

pub struct PendingConnection {
    pub handle: std::thread::JoinHandle<Result<(SocketAddr, SocketAddr), String>>,
    pub peer_addr: SocketAddr,
}

pub struct NetworkSystem {
    pub status: NetworkStatus,
    pub is_host: bool,
    pub local_player_id: usize,
    pub session: Option<ggrs::P2PSession<HostConfig>>,
    pub tcp_listener: Option<std::net::TcpListener>,
    pub local_udp_addr: Option<SocketAddr>,
    pub remote_udp_addr: Option<SocketAddr>,
    pub frame_count: u32,
    pub game_state: i32,
    pub last_frame_time: Instant,
    pub event_queue: VecDeque<NetworkEvent>,
    pub consecutive_errors: u32,
    pub sync_start_time: Option<Instant>,
    pub current_pid: u32,
    pub discovery_thread: Option<thread::JoinHandle<DiscoveryResult>>,
    pub discovered_hosts: Arc<Mutex<Vec<HostInfo>>>,
    pub target_host_ip: Option<String>,
    pub broadcast_listener_thread: Option<std::thread::JoinHandle<()>>,
    pub pending_connections: Vec<PendingConnection>,
}

impl NetworkSystem {
    pub fn new(is_host: bool) -> Self {
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
            target_host_ip: None,
            broadcast_listener_thread: None,
            pending_connections: Vec::new(),
        }
    }

    pub fn push_event(&mut self, event: NetworkEvent) {
        self.event_queue.push_back(event);
        // Keep queue size reasonable
        if self.event_queue.len() > 100 {
            self.event_queue.pop_front();
        }
    }
    
    pub fn set_target_host_ip(&mut self, ip: String) {
        self.target_host_ip = Some(ip);
    }

    pub fn setup_ggrs_session(&mut self) -> Result<String, String> {
        if let (Some(local_addr), Some(remote_addr)) = (self.local_udp_addr, self.remote_udp_addr) {
            let debug_msg = format!("Setting up GGRS session - Local: {}, Remote: {}", local_addr, remote_addr);
            
            match UdpNonBlockingSocket::bind_to_port(local_addr.port()) {
                Ok(udp_socket) => {
                    let remote_player_id = if self.is_host { 1 } else { 0 };
                    
                    match SessionBuilder::<HostConfig>::new()
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

    pub fn check_discovery_complete(&mut self) -> bool {
        if let Some(handle) = self.discovery_thread.take() {
            if handle.is_finished() {
                match handle.join() {
                    Ok(result) => {
                        // Store the discovered hosts in the system
                        if let Ok(mut hosts) = self.discovered_hosts.lock() {
                            hosts.clear();
                            hosts.extend(result.hosts.clone());
                        }
                        
                        // Also push the event for immediate notification
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

/// Get the local IP address that can be reached by other devices on the same network
pub fn get_local_ip() -> Result<IpAddr, io::Error> {
    // Try to connect to a remote address to determine our local IP
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect("8.8.8.8:80")?;
    let local_addr = socket.local_addr()?;
    Ok(local_addr.ip())
}

/// Get the local IP as a string, fallback to localhost if detection fails
pub fn get_local_ip_string() -> String {
    match get_local_ip() {
        Ok(ip) => ip.to_string(),
        Err(_) => {
            println!("Warning: Could not detect local IP, falling back to localhost");
            "127.0.0.1".to_string()
        }
    }
}

/// Get broadcast address for the local network
pub fn get_broadcast_address(local_ip: &str) -> String {
    if let Ok(ip) = local_ip.parse::<Ipv4Addr>() {
        let octets = ip.octets();
        // Assume /24 subnet (common for home networks)
        format!("{}.{}.{}.255", octets[0], octets[1], octets[2])
    } else {
        // Fallback to limited broadcast
        "255.255.255.255".to_string()
    }
}

/// Create a socket address using the local IP
pub fn create_local_socket_addr(port: u16) -> Result<SocketAddr, io::Error> {
    let local_ip = get_local_ip()?;
    Ok(SocketAddr::new(local_ip, port))
}
/*
/// Get all available network interfaces (more comprehensive approach)
pub fn get_network_interfaces() -> Result<Vec<IpAddr>, Box<dyn std::error::Error>> {
    let network_interfaces = NetworkInterface::show()?;
    let mut ips = Vec::new();
    
    for interface in network_interfaces {
        // Skip loopback and non-running interfaces
        if interface.name.starts_with("loop") || !interface.flags.is_running() {
            continue;
        }
        
        for addr in interface.addr {
            match addr.ip() {
                IpAddr::V4(ipv4) => {
                    // Skip localhost and link-local addresses
                    if !ipv4.is_loopback() && !ipv4.is_link_local() {
                        ips.push(IpAddr::V4(ipv4));
                    }
                }
                IpAddr::V6(ipv6) => {
                    // Skip localhost and link-local IPv6 addresses
                    if !ipv6.is_loopback() && !ipv6.is_unicast_link_local() {
                        ips.push(IpAddr::V6(ipv6));
                    }
                }
            }
        }
    }
    
    Ok(ips)
}
*/
/// Simple way to detect if we're on a local network
pub fn is_local_network_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            ipv4.is_private() && !ipv4.is_loopback()
        }
        IpAddr::V6(ipv6) => {
            ipv6.is_unique_local() && !ipv6.is_loopback()
        }
    }
}


pub fn get_local_devices_via_arp() -> Vec<String> {
    let output = if cfg!(target_os = "windows") {
        Command::new("arp").arg("-a").output().ok()
    } else {
        Command::new("arp").arg("-an").output().ok()
    };

    let mut devices = Vec::new();
    
    if let Some(output) = output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if let Some(ip) = extract_ip_from_arp_line(line) {
                    devices.push(ip);
                }
            }
        }
    }
    
    devices
}

fn extract_ip_from_arp_line(line: &str) -> Option<String> {
    if cfg!(target_os = "windows") {
        // Windows arp -a format: "  192.168.1.1          00-11-22-33-44-55     dynamic"
        line.split_whitespace().nth(1).map(|s| s.to_string())
    } else {
        // Linux/Mac arp -an format: "? (192.168.1.1) at 00:11:22:33:44:55 [ether] on eth0"
        line.split('(').nth(1)?.split(')').next().map(|s| s.to_string())
    }
}

use ggrs::{Config, SessionBuilder, UdpNonBlockingSocket, PlayerType};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Instant;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread;

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
pub struct GameConfig;

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

pub struct NetworkSystem {
    pub status: NetworkStatus,
    pub is_host: bool,
    pub local_player_id: usize,
    pub session: Option<ggrs::P2PSession<GameConfig>>,
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
        }
    }

    pub fn push_event(&mut self, event: NetworkEvent) {
        self.event_queue.push_back(event);
        // Keep queue size reasonable
        if self.event_queue.len() > 100 {
            self.event_queue.pop_front();
        }
    }

    pub fn setup_ggrs_session(&mut self) -> Result<String, String> {
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
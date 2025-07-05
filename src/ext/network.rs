use std::io::BufRead;
use ggrs::{Config, PlayerType, SessionBuilder, UdpNonBlockingSocket, SessionState};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use std::sync::{atomic, Arc};
use std::io::{self, Write};

#[derive(Debug, Serialize, Deserialize)]
enum NetworkMessage {
    PeerAddress(SocketAddr),
    Ping,
    Pong,
    JoinRequest(u32), // PID of requesting peer
    JoinResponse(SocketAddr), // Host's UDP address
}
#[derive(Debug)]
struct GameConfig;
impl Config for GameConfig {
    type Input = i32;
    type State = i32;
    type Address = SocketAddr;
}
/// Custom waker implementation for manual future polling
struct ManualWaker {
    wake_flag: atomic::AtomicBool,
}
impl std::task::Wake for ManualWaker {
    #[inline]
    fn wake(self: Arc<Self>) {
        self.wake_flag.store(true, atomic::Ordering::Relaxed);
    }
    #[inline]
    fn wake_by_ref(self: &Arc<ManualWaker>) {
        self.wake_flag.store(true, atomic::Ordering::Relaxed);
    }
}

/// Simple TCP-based peer discovery for local network
#[allow(dead_code)]
struct PeerDiscovery {
    port: u16,
    is_host: bool,
    pid: u32,
}

impl PeerDiscovery {
    fn new(is_host: bool, pid: u32) -> Self {
        let port = if is_host { 9000 } else { 9001 + (pid % 100) as u16 };
        Self { port, is_host, pid }
    }

    fn start_host(&self) -> io::Result<(SocketAddr, SocketAddr)> {
        use std::net::TcpListener;
        use std::io::{BufRead, BufReader};

        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.port))?;
        println!("Host listening on port {}", self.port);

        // Create UDP socket for game traffic
        let udp_port = 7000 + (self.pid % 1000) as u16;
        let _udp_socket = UdpNonBlockingSocket::bind_to_port(udp_port)
            .map_err(|_| io::Error::new(io::ErrorKind::AddrInUse, "UDP port busy"))?;
        let udp_addr = SocketAddr::new(
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
            udp_port,
        );

        // Wait for peer connection
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    {
                        // Create a new scope for the reader
                        let mut reader = BufReader::new(&stream);
                        let mut line = String::new();
                        
                        if reader.read_line(&mut line).is_ok() {
                            if let Ok(msg) = serde_json::from_str::<NetworkMessage>(&line.trim()) {
                                match msg {
                                    NetworkMessage::JoinRequest(peer_pid) => {
                                        println!("Received join request from peer PID: {}", peer_pid);
                                        
                                        // Drop the reader so we can use stream mutably
                                        drop(reader);
                                        
                                        // Send our UDP address back
                                        let response = NetworkMessage::JoinResponse(udp_addr);
                                        let response_json = serde_json::to_string(&response).unwrap();
                                        writeln!(stream, "{}", response_json).ok();
                                        
                                        // Create a new reader for the next read
                                        let mut reader = BufReader::new(&stream);
                                        
                                        // Wait for peer's UDP address
                                        let mut peer_line = String::new();
                                        if reader.read_line(&mut peer_line).is_ok() {
                                            if let Ok(peer_msg) = serde_json::from_str::<NetworkMessage>(&peer_line.trim()) {
                                                match peer_msg {
                                                    NetworkMessage::PeerAddress(peer_udp_addr) => {
                                                        println!("Received peer UDP address: {}", peer_udp_addr);
                                                        return Ok((udp_addr, peer_udp_addr));
                                                    }
                                                    _ => {}
                                                }
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("❌Connection error: {}", e);
                }
            }
        }
        
        Err(io::Error::new(io::ErrorKind::ConnectionAborted, "No peer connected"))
    }

    fn connect_to_host(&self, host_pid: u32) -> io::Result<(SocketAddr, SocketAddr)> {
        use std::net::TcpStream;
        use std::io::{BufRead, BufReader};

        let host_port = 9000;
        let host_addr = format!("127.0.0.1:{}", host_port);
        
        println!("Attempting to connect to host PID {} at {}", host_pid, host_addr);
        
        // Create our UDP socket first
        let udp_port = 7000 + (self.pid % 1000) as u16;
        let _udp_socket = UdpNonBlockingSocket::bind_to_port(udp_port)
            .map_err(|_| io::Error::new(io::ErrorKind::AddrInUse, "UDP port busy"))?;
        let local_udp_addr = SocketAddr::new(
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
            udp_port,
        );

        // Connect to host
        let mut stream = TcpStream::connect(&host_addr)
            .map_err(|e| {
                println!("❌Failed to connect to host: {}", e);
                e
            })?;

        // Send join request
        let join_msg = NetworkMessage::JoinRequest(self.pid);
        let join_json = serde_json::to_string(&join_msg).unwrap();
        writeln!(stream, "{}", join_json)?;
        
        // Wait for response
        let mut reader = BufReader::new(&stream);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        
        if let Ok(msg) = serde_json::from_str::<NetworkMessage>(&line.trim()) {
            match msg {
                NetworkMessage::JoinResponse(host_udp_addr) => {
                    println!("Received host UDP address: {}", host_udp_addr);
                    
                    // Send our UDP address to host
                    let peer_msg = NetworkMessage::PeerAddress(local_udp_addr);
                    let peer_json = serde_json::to_string(&peer_msg).unwrap();
                    writeln!(stream, "{}", peer_json)?;
                    
                    println!("Sent our UDP address {} to host", local_udp_addr);
                    return Ok((local_udp_addr, host_udp_addr));
                }
                _ => {}
            }
        }
        
        Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid response from host"))
    }

}

fn run_game_loop(mut session: ggrs::P2PSession<GameConfig>, local_player_id: usize) {
    println!("Starting game loop...");
    println!("You'll see state updates every second");
    println!("Local player ID: {}", local_player_id);
    
    let mut frame_count = 0;
    let mut last_update = Instant::now();
    let frame_duration = Duration::from_secs_f32(1.0 / 60.0);
        
    // Wait for synchronization with more thorough checking
    println!("Waiting for peer synchronization...");
    let sync_start = Instant::now();
    let sync_timeout = Duration::from_secs(15);
    
    // Wait for session to be ready with better polling
    loop {
        session.poll_remote_clients();
        
        // Check if we're synchronized
        match session.current_state() {
            SessionState::Running => {
                println!("✅Session is running and synchronized!");
                break;
            }
            SessionState::Synchronizing => {
                // Still synchronizing, keep waiting
                if sync_start.elapsed() > sync_timeout {
                    println!("⚠️Sync timeout reached, but continuing...");
                    break;
                }
            }
        }
        
        std::thread::sleep(Duration::from_millis(16)); // ~60 FPS
    }
    
    println!("Game loop synchronized and running!\n");
    
    // Game state - simple counter that both peers should maintain
    let mut game_state = 0i32;
    let mut consecutive_errors;
    let max_consecutive_errors = 10;
    
    loop {
        // Handle network events first
        session.poll_remote_clients();
        
        // Check session state for critical issues
        match session.current_state() {
            SessionState::Synchronizing => {
                println!("⚠️Session went back to synchronizing, waiting...");
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            SessionState::Running => {
                // Good, continue with game loop
                consecutive_errors = 0;
            }
        }
        
        frame_count += 1;
        
        // Generate simple input - use a simpler pattern
        let input = (frame_count % 5) as i32;

        // Add local input with better error handling
        match session.add_local_input(local_player_id, input) {
            Ok(_) => {},
            Err(e) => {
                println!("❌Error adding local input for player '{}' : {:?}", local_player_id, e);
                consecutive_errors += 1;
                if consecutive_errors > max_consecutive_errors {
                    println!("❌Too many consecutive errors, exiting...");
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
        }

        // Advance frame with improved error handling
        match session.advance_frame() {
            Ok(_) => {
                // Update game state - this is where you'd run your game logic
                game_state += 1;
            },
            Err(e) => {
                consecutive_errors += 1;
                
                match e {
                    ggrs::GgrsError::PredictionThreshold => {
                        println!("⚠️Prediction threshold reached");
                        // More aggressive slowdown when prediction threshold is hit
                        std::thread::sleep(Duration::from_millis(200));
                        
                        // If we hit prediction threshold too many times, something is wrong
                        if consecutive_errors > max_consecutive_errors {
                            println!("❌Too many prediction threshold errors, network may be unstable");
                            return;
                        }
                    }
                    ggrs::GgrsError::NotSynchronized => {
                        println!("⚠️Not synchronized");
                        std::thread::sleep(Duration::from_millis(100));
                    }
                    _ => {
                        println!("❌Error advancing frame: {:?}", e);
                        std::thread::sleep(Duration::from_millis(100));
                    }
                }
                continue;
            }
        }

        // Print state every second
        if frame_count % 60 == 0 {
            let session_state = session.current_state();
            println!("Frame {}: Game State: {}, Session: {:?}", frame_count, game_state, session_state);
        }

        // Maintain frame rate but be more lenient
        let elapsed = last_update.elapsed();
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
        last_update = Instant::now();
    }
}

/// Discovers potential hosts in the network by scanning common ports
pub fn discover_hosts(timeout_ms: u64) -> Vec<(u32, SocketAddr)> {
    use std::net::{TcpStream, IpAddr, Ipv4Addr};
    use std::time::Duration;

    println!("Scanning for potential hosts...");
    let mut hosts = Vec::new();

    // Scan a range of possible host ports (9000-9009 in this example)
    for port in 9000..=9009 {
        let addr = SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port,
        );

        // Try to connect with a short timeout
        if let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(timeout_ms)) {
            println!("Found potential host at {}", addr);

            // Send a discovery message
            let discovery_msg = NetworkMessage::Ping;
            let discovery_json = serde_json::to_string(&discovery_msg).unwrap();
            if writeln!(stream, "{}", discovery_json).is_ok() {
                // Read response to get PID
                let mut reader = std::io::BufReader::new(&stream);
                let mut line = String::new();
                if reader.read_line(&mut line).is_ok() {
                    if let Ok(NetworkMessage::JoinRequest(pid)) = serde_json::from_str(&line.trim()) {
                        hosts.push((pid, addr));
                    }
                }
            }
        }
    }

    hosts
}

// will rework it to make it completely different for host and peer
pub fn initialize(is_host: bool, target_pid: u32) {
    
    let current_pid = std::process::id();
    
    let discovery = PeerDiscovery::new(is_host, current_pid);
    
    let (local_udp_addr, remote_udp_addr) = if is_host {
        println!("\nStarting as HOST...");
        match discovery.start_host() {
            Ok((local_addr, remote_addr)) => (local_addr, remote_addr),
            Err(e) => {
                println!("❌Failed to start host: {}", e);
                return;
            }
        }
    } else {
        println!("\nStarting as PEER...");
        match discovery.connect_to_host(target_pid) {
            Ok((local_addr, remote_addr)) => (local_addr, remote_addr),
            Err(e) => {
                println!("❌ Failed to connect to host: {}", e);
                return;
            }
        }
    };

    println!("\nSetting up GGRS session...");
    println!("Local UDP:  {}, Remote UDP: {}", local_udp_addr, remote_udp_addr);

    // Create UDP socket for GGRS
    let udp_socket = match UdpNonBlockingSocket::bind_to_port(local_udp_addr.port()) {
        Ok(socket) => socket,
        Err(e) => {
            println!("❌Failed to bind UDP socket: {:?}", e);
            return;
        }
    };

    // FIXED: Properly assign player IDs
    let local_player_id = if is_host { 0 } else { 1 };
    let remote_player_id = if is_host { 1 } else { 0 };
    
    println!("Local player ID: {}, Remote player ID: {}", local_player_id, remote_player_id);

    // adding players should be handled a bit differently so logging in-out could actually work, while the host still continues
    
    let session = match SessionBuilder::<GameConfig>::new()
        .with_num_players(2)
        .with_input_delay(2) // Increased input delay for better stability
        .with_fps(60)
        .unwrap()
        .with_max_prediction_window(6) // Reduced prediction window to prevent runaway prediction
        .expect("❌Failed to set prediction_window")
        .with_check_distance(30) // Add check distance for better desync detection
        .add_player(PlayerType::Local, local_player_id)
        .unwrap()
        .add_player(PlayerType::Remote(remote_udp_addr), remote_player_id)
        .unwrap()
        .start_p2p_session(udp_socket)
    {
        Ok(session) => session,
        Err(e) => {
            println!("❌Failed to create GGRS session: {:?}", e);
            return;
        }
    };

    println!("GGRS session created successfully!");
    
    run_game_loop(session, local_player_id);
}



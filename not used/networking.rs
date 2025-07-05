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
        println!("🏠 Host listening on port {}", self.port);
        println!("📡 Waiting for peer to connect...");

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
                                        println!("📨 Received join request from peer PID: {}", peer_pid);
                                        
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
                                                        println!("✅ Received peer UDP address: {}", peer_udp_addr);
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
                    println!("❌ Connection error: {}", e);
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
        
        println!("🔗 Attempting to connect to host PID {} at {}", host_pid, host_addr);
        
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
                println!("❌ Failed to connect to host: {}", e);
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
                    println!("✅ Received host UDP address: {}", host_udp_addr);
                    
                    // Send our UDP address to host
                    let peer_msg = NetworkMessage::PeerAddress(local_udp_addr);
                    let peer_json = serde_json::to_string(&peer_msg).unwrap();
                    writeln!(stream, "{}", peer_json)?;
                    
                    println!("✅ Sent our UDP address {} to host", local_udp_addr);
                    return Ok((local_udp_addr, host_udp_addr));
                }
                _ => {}
            }
        }
        
        Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid response from host"))
    }
}

fn get_user_choice() -> (bool, Option<u32>) {
    println!("\n🎮 Choose your role:");
    println!("1. Host (wait for others to connect)");
    println!("2. Peer (connect to existing host)");
    
    print!("Enter your choice (1 or 2): ");
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    
    match input.trim() {
        "1" => {
            println!("🏠 You selected: HOST");
            (true, None)
        }
        "2" => {
            println!("🔗 You selected: PEER");
            print!("Enter the PID of the host to connect to: ");
            io::stdout().flush().unwrap();
            
            let mut pid_input = String::new();
            io::stdin().read_line(&mut pid_input).unwrap();
            
            match pid_input.trim().parse::<u32>() {
                Ok(pid) => (false, Some(pid)),
                Err(_) => {
                    println!("❌ Invalid PID. Using default connection attempt.");
                    (false, Some(0))
                }
            }
        }
        _ => {
            println!("❌ Invalid choice. Defaulting to HOST mode.");
            (true, None)
        }
    }
}

/// Error throttling helper
struct ErrorThrottler {
    last_error_time: Instant,
    error_interval: Duration,
    error_count: u32,
}

impl ErrorThrottler {
    fn new(interval_ms: u64) -> Self {
        Self {
            last_error_time: Instant::now() - Duration::from_millis(interval_ms),
            error_interval: Duration::from_millis(interval_ms),
            error_count: 0,
        }
    }

    fn should_print(&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since(self.last_error_time) >= self.error_interval {
            self.last_error_time = now;
            let should_print = self.error_count > 0;
            self.error_count = 0;
            should_print
        } else {
            self.error_count += 1;
            false
        }
    }

    fn get_count(&self) -> u32 {
        self.error_count
    }
}

// FIXED: Pass local_player_id to the game loop
fn run_game_loop(mut session: ggrs::P2PSession<GameConfig>, local_player_id: usize) {
    println!("\n🎮 Starting game loop...");
    println!("📊 You'll see state updates every second");
    println!("👤 Local player ID: {}", local_player_id);
    println!("⏹️  Press Ctrl+C to stop\n");
    
    let mut frame_count = 0;
    let mut last_update = Instant::now();
    let frame_duration = Duration::from_secs_f32(1.0 / 60.0);
    
    // Error throttling - only print errors every 500ms
    let mut error_throttler = ErrorThrottler::new(500);
    
    // Wait for synchronization with more thorough checking
    println!("🔄 Waiting for peer synchronization...");
    let sync_start = Instant::now();
    let sync_timeout = Duration::from_secs(15);
    
    // Wait for session to be ready with better polling
    loop {
        session.poll_remote_clients();
        
        // Check if we're synchronized
        match session.current_state() {
            SessionState::Running => {
                println!("✅ Session is running and synchronized!");
                break;
            }
            SessionState::Synchronizing => {
                // Still synchronizing, keep waiting
                if sync_start.elapsed() > sync_timeout {
                    println!("⚠️  Sync timeout reached, but continuing...");
                    break;
                }
            }
        }
        
        std::thread::sleep(Duration::from_millis(16)); // ~60 FPS
    }
    
    println!("🎮 Game loop synchronized and running!\n");
    
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
                println!("⚠️  Session went back to synchronizing, waiting...");
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
                if error_throttler.should_print() {
                    println!("❌ Error adding local input for player {}: {:?} ({}x)", local_player_id, e, error_throttler.get_count());
                }
                consecutive_errors += 1;
                if consecutive_errors > max_consecutive_errors {
                    println!("❌ Too many consecutive errors, exiting...");
                    return;
                }
                std::thread::sleep(Duration::from_millis(50));
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
                        if error_throttler.should_print() {
                            println!("⚠️  Prediction threshold reached - slowing down ({}x)", error_throttler.get_count());
                        }
                        // More aggressive slowdown when prediction threshold is hit
                        std::thread::sleep(Duration::from_millis(200));
                        
                        // If we hit prediction threshold too many times, something is wrong
                        if consecutive_errors > max_consecutive_errors {
                            println!("❌ Too many prediction threshold errors, network may be unstable");
                            return;
                        }
                    }
                    ggrs::GgrsError::NotSynchronized => {
                        if error_throttler.should_print() {
                            println!("⚠️  Not synchronized - waiting ({}x)", error_throttler.get_count());
                        }
                        std::thread::sleep(Duration::from_millis(100));
                    }
                    _ => {
                        if error_throttler.should_print() {
                            println!("❌ Error advancing frame: {:?} ({}x)", e, error_throttler.get_count());
                        }
                        std::thread::sleep(Duration::from_millis(100));
                    }
                }
                continue;
            }
        }

        // Print state every second
        if frame_count % 60 == 0 {
            let session_state = session.current_state();
            println!("📈 Frame {}: Game State: {}, Session: {:?}", frame_count, game_state, session_state);
        }

        // Maintain frame rate but be more lenient
        let elapsed = last_update.elapsed();
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
        last_update = Instant::now();
    }
}

fn main() {
    println!("=== P2P Networking App ===");
    println!("Starting instance with PID: {}", std::process::id());
    println!("🌐 Local network P2P connection");
    println!("==============================");
    
    let current_pid = std::process::id();
    let (is_host, target_pid) = get_user_choice();
    
    let discovery = PeerDiscovery::new(is_host, current_pid);
    
    let (local_udp_addr, remote_udp_addr) = if is_host {
        println!("\n🏠 Starting as HOST...");
        match discovery.start_host() {
            Ok((local_addr, remote_addr)) => (local_addr, remote_addr),
            Err(e) => {
                println!("❌ Failed to start host: {}", e);
                return;
            }
        }
    } else {
        println!("\n🔗 Starting as PEER...");
        let host_pid = target_pid.unwrap_or(0);
        match discovery.connect_to_host(host_pid) {
            Ok((local_addr, remote_addr)) => (local_addr, remote_addr),
            Err(e) => {
                println!("❌ Failed to connect to host: {}", e);
                return;
            }
        }
    };

    println!("\n🔧 Setting up GGRS session...");
    println!("📍 Local UDP:  {}", local_udp_addr);
    println!("📍 Remote UDP: {}", remote_udp_addr);

    // Create UDP socket for GGRS
    let udp_socket = match UdpNonBlockingSocket::bind_to_port(local_udp_addr.port()) {
        Ok(socket) => socket,
        Err(e) => {
            println!("❌ Failed to bind UDP socket: {:?}", e);
            return;
        }
    };

    // FIXED: Properly assign player IDs
    let local_player_id = if is_host { 0 } else { 1 };
    let remote_player_id = if is_host { 1 } else { 0 };
    
    println!("👤 Local player ID: {}, Remote player ID: {}", local_player_id, remote_player_id);
    
    let session = match SessionBuilder::<GameConfig>::new()
        .with_num_players(2)
        .with_input_delay(2) // Increased input delay for better stability
        .with_fps(60)
        .unwrap()
        .with_max_prediction_window(6) // Reduced prediction window to prevent runaway prediction
        .expect("❌ Failed to set prediction_window")
        .with_check_distance(3) // Add check distance for better desync detection
        .add_player(PlayerType::Local, local_player_id)
        .unwrap()
        .add_player(PlayerType::Remote(remote_udp_addr), remote_player_id)
        .unwrap()
        .start_p2p_session(udp_socket)
    {
        Ok(session) => session,
        Err(e) => {
            println!("❌ Failed to create GGRS session: {:?}", e);
            return;
        }
    };

    println!("✅ GGRS session created successfully!");
    
    // FIXED: Pass the local player ID to the game loop
    run_game_loop(session, local_player_id);
}
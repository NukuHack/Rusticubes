
use crate::network::types::{self, NetworkMessage, PendingConnection, NetworkEvent, NetworkStatus, NetworkSystem, DiscoveryResult, HostInfo};
use crate::network::api;
use crate::ext::config;
use std::io::{self, BufRead, BufReader, Write};
use std::net::{SocketAddr, IpAddr, TcpListener, TcpStream, UdpSocket};
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use std::thread;


const TCP_PORT: u16 = 9000;
const DISCOVERY_PORT: u16 = 9001; // Separate port for discovery

// New message types for broadcast discovery
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum BroadcastMessage {
    DiscoveryRequest {
        port: u16,
        sender_ip: String,
    },
    DiscoveryResponse {
        ip: String,
        port: u16,
    },
}


/// Updated main discovery function to use broadcast
#[inline]
pub fn discover_hosts(timeout_ms: u64) -> Result<String, String> {
    let system = api::get_ptr().ok_or("Network system not initialized")?;
    
    let handle = thread::spawn(move || {
        discover_hosts_broadcast(timeout_ms)
    });
    
    system.discovery_thread = Some(handle);
    Ok(format!("Broadcast discovery started with timeout: {}ms", timeout_ms))
}

impl NetworkSystem {
    pub fn start_discovery_thread(&mut self, timeout_ms: u64) {
        let handle = thread::spawn(move || {
            discover_hosts_threaded(timeout_ms)
        });
        
        self.discovery_thread = Some(handle);
    }

    /// Start UDP broadcast listener in background thread
    pub fn start_broadcast_listener(&mut self) -> Result<String, String> {
        if !self.is_host {
            return Ok("Not host, no broadcast listener needed".to_string());
        }

        let local_ip = types::get_local_ip_string();
        let handle = thread::spawn(move || {
            broadcast_listener_thread(local_ip);
        });
        
        self.broadcast_listener_thread = Some(handle);
        Ok("Broadcast listener started".to_string())
    }

    pub fn setup_tcp_listener(&mut self) -> Result<String, String> {
        if !self.is_host { return Ok("Not host, no TCP listener needed".to_string()); }
        
        if self.tcp_listener.is_some() { return Ok("TCP listener already exists".to_string()); }
        
        let addr = format!("0.0.0.0:{}", TCP_PORT);
        
        let listener = TcpListener::bind(&addr).map_err(|e| format!("Failed to bind TCP listener to {}: {}", addr, e))?;
        
        listener.set_nonblocking(true).map_err(|e| format!("Failed to set TCP listener non-blocking: {}", e))?;
        
        self.tcp_listener = Some(listener);
        
        let local_ip = types::get_local_ip_string();
        Ok(format!("Successfully bound TCP listener to {} - Connect using: {}:{}", addr, local_ip, TCP_PORT))
    }

    pub fn handle_client_message(&mut self, stream: &mut TcpStream, message: NetworkMessage) -> Result<(bool, String), String> {
        match message {
            NetworkMessage::WorldInfoRequest => {
                self.handle_world_info_request(stream)
            }
            NetworkMessage::JoinRequest(peer_pid) => {
                self.handle_join_request(stream, peer_pid)
            }
            _ => Ok((false, "Received other message type".to_string()))
        }
    }

    fn handle_world_info_request(&mut self, stream: &mut TcpStream) -> Result<(bool, String), String> {
        let world_name = config::get_gamestate().worldname().to_string();
        let response = NetworkMessage::WorldInfoResponse(world_name.clone());
        
        let response_json = serde_json::to_string(&response).map_err(|e| format!("Failed to serialize world info response: {}", e))?;
        
        writeln!(stream, "{}", response_json).map_err(|e| format!("Failed to write world info response: {}", e))?;
        
        Ok((false, format!("Sent world info response: {}", world_name)))
    }

    fn handle_join_request(&mut self, stream: &mut TcpStream, peer_pid: u32) -> Result<(bool, String), String> {
        let debug_msg = format!("Received join request from peer PID: {}", peer_pid);
        
        // CHANGE: Use local IP instead of hardcoded localhost
        let local_ip = types::get_local_ip().map_err(|e| format!("Failed to get local IP: {}", e))?;
        
        let udp_port = 7000 + (self.current_pid % 1000) as u16;
        let udp_addr = SocketAddr::new(local_ip, udp_port);
        
        // Send response
        let response = NetworkMessage::JoinResponse(udp_addr);
        let response_json = serde_json::to_string(&response).map_err(|e| format!("{} - Failed to serialize join response: {}", debug_msg, e))?;
        
        writeln!(stream, "{}", response_json).map_err(|e| format!("{} - Failed to write join response: {}", debug_msg, e))?;
        
        // Read peer's UDP address
        let mut reader = BufReader::new(stream);
        let mut peer_line = String::new();
        
        reader.read_line(&mut peer_line).map_err(|e| format!("{} - Failed to read peer UDP address: {}", debug_msg, e))?;
        
        let peer_msg: NetworkMessage = serde_json::from_str(&peer_line.trim()).map_err(|e| format!("{} - Failed to parse peer message: {}", debug_msg, e))?;
        
        if let NetworkMessage::PeerAddress(peer_udp_addr) = peer_msg {
            self.local_udp_addr = Some(udp_addr);
            self.remote_udp_addr = Some(peer_udp_addr);
            Ok((true, format!("{} - Successfully exchanged UDP addresses", debug_msg)))
        } else {
            Err(format!("{} - Received wrong message type from peer", debug_msg))
        }
    }

    pub fn try_accept_connection(&mut self) -> Result<(bool, String), String> {
        let listener = match &self.tcp_listener {
            Some(listener) => listener,
            None => return Ok((false, "No TCP listener available".to_string())),
        };
        
        let (stream, addr) = match listener.accept() {
            Ok(result) => result,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                return Ok((false, "No connection available".to_string()));
            }
            Err(e) => return Err(format!("Accept error: {}", e)),
        };
        
        let debug_msg = format!("Accepted connection from {}", addr);
        
        // Spawn a thread to handle the blocking handshake
        let current_pid = self.current_pid;
        let local_ip = types::get_local_ip().map_err(|e| format!("Failed to get local IP: {}", e))?;
        
        let handle = std::thread::spawn(move || {
            Self::handle_host_handshake(stream, addr, current_pid, local_ip)
        });
        
        self.pending_connections.push(PendingConnection {
            handle,
            peer_addr: addr,
        });
        
        Ok((false, format!("{} - Handshake started in background", debug_msg)))
    }

    fn handle_host_handshake(
        mut stream: TcpStream, 
        peer_addr: SocketAddr,
        current_pid: u32,
        local_ip: IpAddr
    ) -> Result<(SocketAddr, SocketAddr), String> {
        let debug_msg = format!("Handling handshake with {}", peer_addr);
        
        // Set a reasonable timeout for the handshake
        stream.set_read_timeout(Some(Duration::from_secs(10)))
            .map_err(|e| format!("{} - Failed to set read timeout: {}", debug_msg, e))?;
        
        // Read the join request
        let mut reader = BufReader::new(&mut stream);
        let mut line = String::new();
        
        reader.read_line(&mut line)
            .map_err(|e| format!("{} - Failed to read join request: {}", debug_msg, e))?;
        
        let msg: NetworkMessage = serde_json::from_str(&line.trim())
            .map_err(|e| format!("{} - Failed to parse join request: {}", debug_msg, e))?;
        
        let peer_pid = match msg {
            NetworkMessage::JoinRequest(pid) => pid,
            _ => return Err(format!("{} - Received wrong message type", debug_msg)),
        };
        
        let debug_msg = format!("{} - Received join request from peer PID: {}", debug_msg, peer_pid);
        
        // Calculate UDP addresses
        let udp_port = 7000 + (current_pid % 1000) as u16;
        let local_udp_addr = SocketAddr::new(local_ip, udp_port);
        
        // Send join response
        let response = NetworkMessage::JoinResponse(local_udp_addr);
        let response_json = serde_json::to_string(&response)
            .map_err(|e| format!("{} - Failed to serialize join response: {}", debug_msg, e))?;
        
        drop(reader); // Release the reader so we can use stream mutably
        
        writeln!(stream, "{}", response_json)
            .map_err(|e| format!("{} - Failed to write join response: {}", debug_msg, e))?;
        
        // Read peer's UDP address
        let mut reader = BufReader::new(&mut stream);
        let mut peer_line = String::new();
        
        reader.read_line(&mut peer_line)
            .map_err(|e| format!("{} - Failed to read peer UDP address: {}", debug_msg, e))?;
        
        let peer_msg: NetworkMessage = serde_json::from_str(&peer_line.trim())
            .map_err(|e| format!("{} - Failed to parse peer message: {}", debug_msg, e))?;
        
        let peer_udp_addr = match peer_msg {
            NetworkMessage::PeerAddress(addr) => addr,
            _ => return Err(format!("{} - Received wrong message type from peer", debug_msg)),
        };
        
        Ok((local_udp_addr, peer_udp_addr))
    }

    pub fn try_connect_to_host(&mut self, target_host_ip: &str) -> Result<(bool, String), String> {
        let host_addr = format!("{}:{}", target_host_ip, TCP_PORT);
        
        // Spawn a thread to handle the blocking connection
        let current_pid = self.current_pid;
        let local_ip = types::get_local_ip().map_err(|e| format!("Failed to get local IP: {}", e))?;
        let host_addr_clone = host_addr.clone();
        
        let handle = std::thread::spawn(move || {
            Self::handle_client_handshake(host_addr_clone, current_pid, local_ip)
        });
        
        self.pending_connections.push(PendingConnection {
            handle,
            peer_addr: host_addr.parse().map_err(|e| format!("Invalid host address: {}", e))?,
        });
        
        Ok((false, format!("Connection to {} started in background", host_addr)))
    }

    fn handle_client_handshake(
        host_addr: String,
        current_pid: u32,
        local_ip: IpAddr
    ) -> Result<(SocketAddr, SocketAddr), String> {
        let debug_msg = format!("Connecting to host at {}", host_addr);
        
        let mut stream = TcpStream::connect_timeout(
            &host_addr.parse().map_err(|e| format!("Invalid address: {}", e))?,
            Duration::from_secs(5)
        ).map_err(|e| format!("{} - Failed to connect: {}", debug_msg, e))?;
        
        // Set timeout for the handshake
        stream.set_read_timeout(Some(Duration::from_secs(10)))
            .map_err(|e| format!("{} - Failed to set read timeout: {}", debug_msg, e))?;
        
        let udp_port = 7000 + (current_pid % 1000) as u16;
        let local_udp_addr = SocketAddr::new(local_ip, udp_port);
        
        // Send join request
        let join_msg = NetworkMessage::JoinRequest(current_pid);
        let join_json = serde_json::to_string(&join_msg)
            .map_err(|e| format!("{} - Failed to serialize join request: {}", debug_msg, e))?;
        
        writeln!(stream, "{}", join_json)
            .map_err(|e| format!("{} - Failed to send join request: {}", debug_msg, e))?;
        
        // Read response
        let mut reader = BufReader::new(&mut stream);
        let mut line = String::new();
        
        reader.read_line(&mut line)
            .map_err(|e| format!("{} - Failed to read host response: {}", debug_msg, e))?;
        
        let msg: NetworkMessage = serde_json::from_str(&line.trim())
            .map_err(|e| format!("{} - Failed to parse host response: {}", debug_msg, e))?;
        
        let host_udp_addr = match msg {
            NetworkMessage::JoinResponse(addr) => addr,
            _ => return Err(format!("{} - Received wrong response type", debug_msg)),
        };
        
        // Send our UDP address
        let peer_msg = NetworkMessage::PeerAddress(local_udp_addr);
        let peer_json = serde_json::to_string(&peer_msg)
            .map_err(|e| format!("{} - Failed to serialize peer address: {}", debug_msg, e))?;
        
        drop(reader); // Release the reader
        
        writeln!(stream, "{}", peer_json)
            .map_err(|e| format!("{} - Failed to send UDP address: {}", debug_msg, e))?;
        
        Ok((local_udp_addr, host_udp_addr))
    }

    // Add this method to check for completed handshakes
    pub fn check_pending_connections(&mut self) -> Result<bool, String> {
        let mut completed_indices = Vec::new();
        
        for (i, pending) in self.pending_connections.iter().enumerate() {
            if pending.handle.is_finished() {
                completed_indices.push(i);
            }
        }
        
        for i in completed_indices.into_iter().rev() {
            let pending = self.pending_connections.remove(i);
            match pending.handle.join() {
                Ok(Ok((local_udp_addr, remote_udp_addr))) => {
                    self.local_udp_addr = Some(local_udp_addr);
                    self.remote_udp_addr = Some(remote_udp_addr);
                    return Ok(true); // Connection successful
                }
                Ok(Err(e)) => {
                    let error_msg = format!("Connection to {} failed: {}", pending.peer_addr, e);
                    self.status = NetworkStatus::Error(error_msg.clone());
                    self.push_event(NetworkEvent::Error(error_msg));
                }
                Err(_) => {
                    let error_msg = format!("Connection thread to {} panicked", pending.peer_addr);
                    self.status = NetworkStatus::Error(error_msg.clone());
                    self.push_event(NetworkEvent::Error(error_msg));
                }
            }
        }
        
        Ok(false) // No connections completed yet
    }
}

/// Background thread that listens for broadcast discovery requests
fn broadcast_listener_thread(local_ip: String) {
    let socket = match UdpSocket::bind(format!("0.0.0.0:{}", DISCOVERY_PORT)) {
        Ok(socket) => socket,
        Err(e) => {
            println!("Failed to bind broadcast listener: {}", e);
            return;
        }
    };

    println!("Broadcast listener started on port {}", DISCOVERY_PORT);

    let mut buf = [0; 1024];
    loop {
        match socket.recv_from(&mut buf) {
            Ok((size, sender_addr)) => {
                let message_str = match std::str::from_utf8(&buf[..size]) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                let request: BroadcastMessage = match serde_json::from_str(message_str) {
                    Ok(msg) => msg,
                    Err(_) => continue,
                };

                if let BroadcastMessage::DiscoveryRequest { port: _, sender_ip: _ } = request {
                    let response = BroadcastMessage::DiscoveryResponse {
                        ip: local_ip.clone(),
                        port: TCP_PORT,
                    };

                    let response_json = match serde_json::to_string(&response) {
                        Ok(json) => json,
                        Err(_) => continue,
                    };

                    // Send response back to the sender
                    if let Err(e) = socket.send_to(response_json.as_bytes(), sender_addr) {
                        println!("Failed to send discovery response: {}", e);
                    } else {
                        println!("Sent discovery response to {}", sender_addr);
                    }
                }
            }
            Err(e) => {
                println!("Error receiving broadcast message: {}", e);
            }
        }
    }
}


// CHANGE: Update discovery function to scan local network
pub fn discover_hosts_threaded(timeout_ms: u64) -> DiscoveryResult {
    let mut hosts = Vec::new();
    let mut debug_info = Vec::new();
    let mut errors = Vec::new();
    
    debug_info.push("Starting host discovery on local network".to_string());
    
    // Get local IP to determine network range
    let local_ip = match types::get_local_ip() {
        Ok(ip) => ip,
        Err(e) => {
            errors.push(format!("Failed to get local IP: {}", e));
            return DiscoveryResult { hosts, debug_info, errors };
        }
    };
    
    debug_info.push(format!("Local IP: {}", local_ip));
    
    // If it's an IPv4 address, scan the local subnet
    if let IpAddr::V4(ipv4) = local_ip {
        let octets = ipv4.octets();
        let network_base = format!("{}.{}.{}", octets[0], octets[1], octets[2]);
        
        debug_info.push(format!("Scanning network: {}.1-254", network_base));
        
        // Scan common IP ranges on the local network
        for host_num in 1..=254 {
            let test_ip = format!("{}.{}", network_base, host_num);
            
            let addr = SocketAddr::new(
                test_ip.parse().unwrap(),
                9000,
            );
            
            if let Some(host_info) = try_discover_host(addr, timeout_ms, &mut debug_info, &mut errors) {
                hosts.push(host_info);
                break; // Found host on this IP, no need to check other ports
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

/// Fixed discover_hosts_broadcast function
pub fn discover_hosts_broadcast(timeout_ms: u64) -> DiscoveryResult {
    let mut hosts = Vec::new();
    let mut debug_info = Vec::new();
    let mut errors = Vec::new();
    
    debug_info.push("Starting UDP broadcast discovery".to_string());
    
    // Get local IP and game name
    let local_ip = match types::get_local_ip_string() {
        ip if !ip.is_empty() => ip,
        _ => {
            errors.push("Failed to get local IP".to_string());
            return DiscoveryResult { hosts, debug_info, errors };
        }
    };
    
    debug_info.push(format!("Local IP: {}", local_ip));
    
    // Create UDP socket for broadcasting
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(socket) => socket,
        Err(e) => {
            errors.push(format!("Failed to create UDP socket: {}", e));
            return DiscoveryResult { hosts, debug_info, errors };
        }
    };
    
    // Enable broadcast
    if let Err(e) = socket.set_broadcast(true) {
        errors.push(format!("Failed to enable broadcast: {}", e));
        return DiscoveryResult { hosts, debug_info, errors };
    }
    
    // Set receive timeout
    if let Err(e) = socket.set_read_timeout(Some(Duration::from_millis(timeout_ms))) {
        errors.push(format!("Failed to set socket timeout: {}", e));
        return DiscoveryResult { hosts, debug_info, errors };
    }
    
    // Create discovery request
    let discovery_request = BroadcastMessage::DiscoveryRequest {
        port: TCP_PORT,
        sender_ip: local_ip.clone(),
    };
    
    let request_json = match serde_json::to_string(&discovery_request) {
        Ok(json) => json,
        Err(e) => {
            errors.push(format!("Failed to serialize discovery request: {}", e));
            return DiscoveryResult { hosts, debug_info, errors };
        }
    };
    
    // Send broadcast to local network
    let broadcast_addr = types::get_broadcast_address(&local_ip);
    let target_addr = format!("{}:{}", broadcast_addr, DISCOVERY_PORT);
    
    debug_info.push(format!("Broadcasting to: {}", target_addr));
    
    if let Err(e) = socket.send_to(request_json.as_bytes(), &target_addr) {
        errors.push(format!("Failed to send broadcast: {}", e));
        return DiscoveryResult { hosts, debug_info, errors };
    }
    
    debug_info.push("Broadcast sent, waiting for responses...".to_string());
    
    // Listen for responses
    let start_time = Instant::now();
    let mut buf = [0; 1024];
    
    while start_time.elapsed() < Duration::from_millis(timeout_ms) {
        match socket.recv_from(&mut buf) {
            Ok((size, sender_addr)) => {
                let message_str = match std::str::from_utf8(&buf[..size]) {
                    Ok(s) => s,
                    Err(e) => {
                        errors.push(format!("Invalid UTF-8 from {}: {}", sender_addr, e));
                        continue;
                    }
                };
                
                debug_info.push(format!("Received response from {}: {}", sender_addr, message_str));
                
                let response: BroadcastMessage = match serde_json::from_str(message_str) {
                    Ok(msg) => msg,
                    Err(e) => {
                        errors.push(format!("Failed to parse response from {}: {}", sender_addr, e));
                        continue;
                    }
                };
                
                if let BroadcastMessage::DiscoveryResponse { ip, port } = response {
                    let host_addr = match format!("{}:{}", ip, port).parse::<SocketAddr>() {
                        Ok(addr) => addr,
                        Err(e) => {
                            errors.push(format!("Invalid address from response: {}:{} - {}", ip, port, e));
                            continue;
                        }
                    };
                    
                    // Try to get the actual world name by connecting to the host
                    let world_name = match get_world_name_from_host(host_addr, &mut debug_info, &mut errors) {
                        Some(name) => name,
                        None => "Unknown".to_string(),
                    };
                    
                    let host_info = HostInfo {
                        pid: port as u32,
                        address: host_addr,
                        world_name,
                    };
                    
                    hosts.push(host_info);
                    debug_info.push(format!("Found host at: {}", host_addr));
                }
            }
            Err(e) if e.kind() == io::ErrorKind::TimedOut => {
                // Timeout is expected, continue until overall timeout
                continue;
            }
            Err(e) => {
                errors.push(format!("Error receiving response: {}", e));
                break;
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

/// Helper function to get world name from a host
fn get_world_name_from_host(addr: SocketAddr, debug_info: &mut Vec<String>, errors: &mut Vec<String>) -> Option<String> {
    let mut stream = match TcpStream::connect_timeout(&addr, Duration::from_millis(1000)) {
        Ok(stream) => stream,
        Err(e) => {
            errors.push(format!("Failed to connect to {} for world info: {}", addr, e));
            return None;
        }
    };
    
    // Request world info
    let world_info_msg = NetworkMessage::WorldInfoRequest;
    let world_info_json = match serde_json::to_string(&world_info_msg) {
        Ok(json) => json,
        Err(e) => {
            errors.push(format!("Failed to serialize world info request for {}: {}", addr, e));
            return None;
        }
    };
    
    if let Err(e) = writeln!(stream, "{}", world_info_json) {
        errors.push(format!("Failed to send world info request to {}: {}", addr, e));
        return None;
    }
    
    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    
    match reader.read_line(&mut line) {
        Ok(_) => {
            match serde_json::from_str::<NetworkMessage>(&line.trim()) {
                Ok(NetworkMessage::WorldInfoResponse(world_name)) => {
                    debug_info.push(format!("Got world name '{}' from {}", world_name, addr));
                    Some(world_name)
                }
                Ok(_) => {
                    errors.push(format!("Received unexpected message type from {}", addr));
                    None
                }
                Err(e) => {
                    errors.push(format!("Failed to parse world info response from {}: {}", addr, e));
                    None
                }
            }
        }
        Err(e) => {
            errors.push(format!("Failed to read world info response from {}: {}", addr, e));
            None
        }
    }
}

fn try_discover_host(addr: SocketAddr, timeout_ms: u64, debug_info: &mut Vec<String>, errors: &mut Vec<String>) -> Option<HostInfo> {
    let mut stream = match TcpStream::connect_timeout(&addr, Duration::from_millis(timeout_ms)) {
        Ok(stream) => {
            debug_info.push(format!("Successfully connected to {}", addr));
            stream
        }
        Err(e) => {
            // Don't log connection refused as errors since that's expected for ports without hosts
            if e.kind() != io::ErrorKind::ConnectionRefused {
                errors.push(format!("Failed to connect to {}: {}", addr, e));
            } else {
                debug_info.push(format!("No host at {}: {}", addr, e));
            }
            return None;
        }
    };
    
    // Request world info
    let world_info_msg = NetworkMessage::WorldInfoRequest;
    let world_info_json = match serde_json::to_string(&world_info_msg) {
        Ok(json) => json,
        Err(e) => {
            errors.push(format!("Failed to serialize world info request for {}: {}", addr, e));
            return None;
        }
    };
    
    debug_info.push(format!("Sending world info request to {}", addr));
    
    if let Err(e) = writeln!(stream, "{}", world_info_json) {
        errors.push(format!("Failed to send world info request to {}: {}", addr, e));
        return None;
    }
    
    debug_info.push(format!("Successfully sent world info request to {}", addr));
    
    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    
    let bytes_read = match reader.read_line(&mut line) {
        Ok(bytes) => bytes,
        Err(e) => {
            errors.push(format!("Failed to read response from {}: {}", addr, e));
            return None;
        }
    };
    
    debug_info.push(format!("Read {} bytes from {}: '{}'", bytes_read, addr, line.trim()));
    
    let world_name = match serde_json::from_str::<NetworkMessage>(&line.trim()) {
        Ok(NetworkMessage::WorldInfoResponse(world_name)) => world_name,
        Ok(other_msg) => {
            errors.push(format!("Received unexpected message type from {}: {:?}", addr, other_msg));
            return None;
        }
        Err(e) => {
            errors.push(format!("Failed to parse response from {}: {} - Raw: '{}'", addr, e, line.trim()));
            return None;
        }
    };
    
    debug_info.push(format!("Host found - PID: {}, Address: {}, World: {}", TCP_PORT, addr, world_name));
    
    Some(HostInfo {
        pid: TCP_PORT as u32,
        address: addr,
        world_name,
    })
}

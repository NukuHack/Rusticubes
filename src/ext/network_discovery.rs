use std::thread;
use crate::ext::network_types::{NetworkMessage, NetworkSystem, DiscoveryResult, HostInfo};
use crate::ext::network_api;
use crate::config;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::Duration;
use std::io;

/// Start discovering hosts in a background thread
#[inline]
pub fn discover_hosts(timeout_ms: u64) -> Result<String, String> {
    if let Some(system) = network_api::get_ptr() {
        system.start_discovery_thread(timeout_ms);
        Ok(format!("Discovery thread started with timeout: {}ms", timeout_ms))
    } else {
        Err("Network system not initialized".to_string())
    }
}

impl NetworkSystem {
    pub fn start_discovery_thread(&mut self, timeout_ms: u64) {
        let handle = thread::spawn(move || {
            discover_hosts_threaded(timeout_ms)
        });
        
        self.discovery_thread = Some(handle);
    }

    pub fn setup_tcp_listener(&mut self) -> Result<String, String> {
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

    pub fn handle_client_message(&mut self, stream: &mut TcpStream, message: NetworkMessage) -> Result<(bool, String), String> {
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

    pub fn try_accept_connection(&mut self) -> Result<(bool, String), String> {
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

    pub fn try_connect_to_host(&mut self, _target_pid: u32) -> Result<(bool, String), String> {
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
}

pub fn discover_hosts_threaded(timeout_ms: u64) -> DiscoveryResult {
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

/// Test if the host is actually responding
#[inline]
pub fn test_host_connection() -> Result<String, String> {
    use std::net::{TcpStream, SocketAddr};
    use std::time::Duration;
    
    let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
    
    match TcpStream::connect_timeout(&addr, Duration::from_millis(1000)) {
        Ok(mut stream) => {
            match stream.set_nonblocking(true) {
                Ok(_) => {
                    let world_info_msg = NetworkMessage::WorldInfoRequest;
                    match serde_json::to_string(&world_info_msg) {
                        Ok(json) => {
                            use std::io::Write;
                            match writeln!(stream, "{}", json) {
                                Ok(_) => {
                                    // Try to read response
                                    use std::io::{BufRead, BufReader};
                                    let mut reader = BufReader::new(&stream);
                                    let mut line = String::new();
                                    match reader.read_line(&mut line) {
                                        Ok(bytes) => Ok(format!("Host responded with {} bytes: '{}'", bytes, line.trim())),
                                        Err(e) => Ok(format!("Host connected but read failed: {}", e))
                                    }
                                }
                                Err(e) => Err(format!("Failed to send request: {}", e))
                            }
                        }
                        Err(e) => Err(format!("Failed to serialize request: {}", e))
                    }
                }
                Err(e) => Err(format!("Connected but failed to set non-blocking: {}", e))
            }
        }
        Err(e) => Err(format!("Failed to connect to host: {}", e))
    }
}
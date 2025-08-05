use crate::{
	ext::ptr,
	network::{
		api,
		types::{self, DiscoveryResult, HostInfo, NetworkEvent, NetworkMessage, 
				NetworkStatus, NetworkSystem, PendingConnection},
	},
};
use std::{
	io::{self, BufRead, BufReader, Write},
	net::{IpAddr, SocketAddr, TcpListener, TcpStream, UdpSocket},
	thread,
	time::{Duration, Instant},
};

const PEER_PORT: u16 = 7000;
const TCP_PORT: u16 = 9000;
const DISCOVERY_PORT: u16 = 9010;

// Custom serialization functions
fn serialize_message(msg: &NetworkMessage) -> Result<String, String> {
	match msg {
		NetworkMessage::WorldInfoRequest => Ok("WORLD_INFO_REQUEST".to_string()),
		NetworkMessage::WorldInfoResponse(world) => Ok(format!("WORLD_INFO_RESPONSE|{}", world)),
		NetworkMessage::JoinRequest(pid) => Ok(format!("JOIN_REQUEST|{}", pid)),
		NetworkMessage::JoinResponse(addr) => Ok(format!("JOIN_RESPONSE|{}", addr)),
		NetworkMessage::PeerAddress(addr) => Ok(format!("PEER_ADDRESS|{}", addr)),
		NetworkMessage::DiscoveryRequest => Ok("DISCOVERY_REQUEST".to_string()),
		NetworkMessage::DiscoveryResponse { ip, port } => Ok(format!("DISCOVERY_RESPONSE|{}|{}", ip, port)),
		NetworkMessage::Ping => Ok("PING".to_string()),
		NetworkMessage::Pong => Ok("PONG".to_string()),
	}
	.map_err(|e: String | format!("Serialization error: {}", e))
}

fn deserialize_message(data: &str) -> Result<NetworkMessage, String> {
	let parts: Vec<&str> = data.trim().split('|').collect();
	
	match parts[0] {
		"WORLD_INFO_REQUEST" => Ok(NetworkMessage::WorldInfoRequest),
		"WORLD_INFO_RESPONSE" => {
			if parts.len() != 2 {
				return Err("Invalid WorldInfoResponse format".to_string());
			}
			Ok(NetworkMessage::WorldInfoResponse(parts[1].to_string()))
		},
		"JOIN_REQUEST" => {
			if parts.len() != 2 {
				return Err("Invalid JoinRequest format".to_string());
			}
			let pid = parts[1].parse::<u32>().map_err(|e| format!("Invalid PID: {}", e))?;
			Ok(NetworkMessage::JoinRequest(pid))
		},
		"JOIN_RESPONSE" => {
			if parts.len() != 2 {
				return Err("Invalid JoinResponse format".to_string());
			}
			let addr = parts[1].parse::<SocketAddr>().map_err(|e| format!("Invalid address: {}", e))?;
			Ok(NetworkMessage::JoinResponse(addr))
		},
		"PEER_ADDRESS" => {
			if parts.len() != 2 {
				return Err("Invalid PeerAddress format".to_string());
			}
			let addr = parts[1].parse::<SocketAddr>().map_err(|e| format!("Invalid address: {}", e))?;
			Ok(NetworkMessage::PeerAddress(addr))
		},
		"DISCOVERY_REQUEST" => Ok(NetworkMessage::DiscoveryRequest),
		"DISCOVERY_RESPONSE" => {
			if parts.len() != 3 {
				return Err("Invalid DiscoveryResponse format".to_string());
			}
			let port = parts[2].parse::<u16>().map_err(|e| format!("Invalid port: {}", e))?;
			Ok(NetworkMessage::DiscoveryResponse {
				ip: parts[1].to_string(),
				port,
			})
		},
		"PING" => {
			Ok(NetworkMessage::Ping)
		},
		"PONG" => {
			Ok(NetworkMessage::Pong)
		},
		_ => Err(format!("Unknown message type: {}", parts[0])),
	}
}

fn serialize_to_bytes(msg: &NetworkMessage) -> Result<Vec<u8>, String> {
	serialize_message(msg).map(|s| s.into_bytes())
}

fn deserialize_from_bytes(data: &[u8]) -> Result<NetworkMessage, String> {
	let s = std::str::from_utf8(data).map_err(|e| format!("UTF-8 error: {}", e))?;
	deserialize_message(s)
}

impl NetworkSystem {
	pub fn start_broadcast_listener(&mut self) -> Result<String, String> {
		if !self.is_host { return Ok("Not host, no broadcast listener needed".into()); }
		
		let local_ip = types::get_local_ip_string();
		self.broadcast_listener_thread = Some(thread::spawn(move || 
			Self::broadcast_listener_thread(local_ip)
		));
		Ok("Broadcast listener started".into())
	}

	pub fn setup_tcp_listener(&mut self) -> Result<String, String> {
		if !self.is_host { return Ok("Not host, no TCP listener needed".into()); }
		if self.tcp_listener.is_some() { return Ok("TCP listener already exists".into()); }
		
		let addr = format!("0.0.0.0:{}", TCP_PORT);
		let listener = TcpListener::bind(&addr).map_err(|e| format!("Failed to bind TCP listener: {}", e))?;
		listener.set_nonblocking(true).map_err(|e| format!("Failed to set non-blocking: {}", e))?;
		
		self.tcp_listener = Some(listener);
		Ok(format!("Bound TCP listener to {}:{}", types::get_local_ip_string(), TCP_PORT))
	}

	pub fn handle_client_message(&mut self, stream: &mut TcpStream, msg: NetworkMessage) -> Result<(bool, String), String> {
		match msg {
			NetworkMessage::WorldInfoRequest => self.handle_world_info_request(stream),
			NetworkMessage::JoinRequest(pid) => self.handle_join_request(stream, pid),
			_ => Ok((false, "Received other message type".into())),
		}
	}

	fn handle_join_request(&mut self, stream: &mut TcpStream, peer_pid: u32) -> Result<(bool, String), String> {
		let debug_msg = format!("Join request from PID: {}", peer_pid);
		let local_ip = types::get_local_ip().map_err(|e| format!("IP error: {}", e))?;
		let udp_port = PEER_PORT + (self.current_pid % 1000) as u16;
		let udp_addr = SocketAddr::new(local_ip, udp_port);
		
		let res = NetworkMessage::JoinResponse(udp_addr);
		writeln!(stream, "{}", serialize_message(&res).map_err(|e| format!("Serialize error: {}", e))?)
			.map_err(|e| format!("Write error: {}", e))?;
		
		let mut reader = BufReader::new(stream);
		let mut line = String::new();
		reader.read_line(&mut line).map_err(|e| format!("Read error: {}", e))?;
		
		if let NetworkMessage::PeerAddress(peer_addr) = deserialize_message(&line.trim())
			.map_err(|e| format!("Parse error: {}", e))? {
			self.local_udp_addr = Some(udp_addr);
			self.remote_udp_addr = Some(peer_addr);
			Ok((true, format!("{} - UDP addresses exchanged", debug_msg)))
		} else {
			Err(format!("{} - Wrong message type", debug_msg))
		}
	}

	pub fn try_connect_to_host(&mut self, target_ip: &str) -> Result<(bool, String), String> {
		let addr = if target_ip.contains(':') {
			target_ip.parse::<SocketAddr>().map_err(|e| format!("Invalid address format: {}", e))?
		} else {
			format!("{}:{}", target_ip, TCP_PORT).parse::<SocketAddr>()
				.map_err(|e| format!("Invalid address format: {}", e))?
		};
		let current_pid = self.current_pid;
		let local_ip = types::get_local_ip().map_err(|e| format!("IP error: {}", e))?;
		
		let handle = thread::spawn(move || {
			Self::handle_client_handshake(addr, current_pid, local_ip)
		});
		
		self.pending_connections.push(PendingConnection { handle, peer_addr: addr });
		Ok((false, format!("Connecting to {}", addr)))
	}

	pub fn try_accept_connection(&mut self) -> Result<(bool, String), String> {
		let listener = match &self.tcp_listener {
			Some(l) => l,
			None => return Ok((false, "No TCP listener".into())),
		};
		
		let (stream, addr) = match listener.accept() {
			Ok(res) => res,
			Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok((false, "No connection".into())),
			Err(e) => return Err(format!("Accept error: {}", e)),
		};
		
		stream.set_nonblocking(true).map_err(|e| format!("Failed to set non-blocking: {}", e))?;
		
		let current_pid = self.current_pid;
		let local_ip = types::get_local_ip().map_err(|e| format!("IP error: {}", e))?;
		
		let handle = thread::spawn(move || {
			Self::handle_host_handshake(stream, addr, current_pid, local_ip)
		});
		
		self.pending_connections.push(PendingConnection { handle, peer_addr: addr });
		Ok((false, format!("Handshake started with {}", addr)))
	}

	fn handle_host_handshake(
		mut stream: TcpStream, 
		_peer_addr: SocketAddr,
		current_pid: u32,
		local_ip: IpAddr
	) -> Result<(SocketAddr, SocketAddr), String> {
		stream.set_read_timeout(Some(Duration::from_millis(1000))).map_err(|e| format!("Timeout error: {}", e))?;
		
		let mut line = String::new();
		
		loop {
			line.clear();
			{
				// Create a new reader in a limited scope
				let mut reader = BufReader::new(&mut stream);
				match reader.read_line(&mut line) {
					Ok(0) => return Err("Connection closed by peer".into()),
					Ok(_) => {},
					Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Err("No data available yet".into()),
					Err(e) => return Err(format!("Read error: {}", e)),
				}
			} // reader is dropped here
			
			let trimmed = line.trim();
			if trimmed.is_empty() { continue; }
			
			match deserialize_message(trimmed).map_err(|e| format!("Parse error: {}", e))? {
				NetworkMessage::WorldInfoRequest => {
					let world = ptr::get_gamestate().worldname().to_string();
					let res = NetworkMessage::WorldInfoResponse(world);
					writeln!(&mut stream, "{}", serialize_message(&res)
						.map_err(|e| format!("Serialize error: {}", e))?)
						.map_err(|e| format!("Write error: {}", e))?;
					stream.flush().map_err(|e| format!("Flush error: {}", e))?;
					continue;
				}
				NetworkMessage::JoinRequest(_peer_pid) => {
					let udp_port = PEER_PORT + (current_pid % 1000) as u16;
					let local_udp_addr = SocketAddr::new(local_ip, udp_port);
					
					let res = NetworkMessage::JoinResponse(local_udp_addr);
					writeln!(&mut stream, "{}", serialize_message(&res).map_err(|e| format!("Serialize error: {}", e))?)
						.map_err(|e| format!("Write error: {}", e))?;
					stream.flush().map_err(|e| format!("Flush error: {}", e))?;
					
					line.clear();
					{
						// Create another reader in a limited scope
						let mut reader = BufReader::new(&mut stream);
						match reader.read_line(&mut line) {
							Ok(0) => return Err("Connection closed by peer".into()),
							Ok(_) => {},
							Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Err("Waiting for peer address".into()),
							Err(e) => return Err(format!("Read error: {}", e)),
						}
					} // reader is dropped here
					
					if let NetworkMessage::PeerAddress(peer_addr) = deserialize_message(&line.trim())
						.map_err(|e| format!("Parse error: {}", e))? {
						return Ok((local_udp_addr, peer_addr));
					} else {
						return Err("Wrong message type for peer address".into());
					}
				}
				_ => return Err("Unexpected message type".into()),
			}
		}
	}

	fn handle_world_info_request(&mut self, stream: &mut TcpStream) -> Result<(bool, String), String> {
		let world = ptr::get_gamestate().worldname().to_string();
		let res = NetworkMessage::WorldInfoResponse(world.clone());
		writeln!(stream, "{}", serialize_message(&res).map_err(|e| format!("Serialize error: {}", e))?)
			.map_err(|e| format!("Write error: {}", e))?;
		stream.flush().map_err(|e| format!("Flush error: {}", e))?;
		Ok((false, format!("Sent world info: {} - connection kept open", world)))
	}
	
	fn handle_client_handshake(
		host_addr: SocketAddr,
		current_pid: u32,
		local_ip: IpAddr
	) -> Result<(SocketAddr, SocketAddr), String> {
		let mut stream = TcpStream::connect_timeout(&host_addr, Duration::from_millis(1000))
			.map_err(|e| format!("Connect error: {}", e))?;
		
		stream.set_read_timeout(Some(Duration::from_millis(1000)))
			.map_err(|e| format!("Timeout error: {}", e))?;
		
		let udp_port = PEER_PORT + (current_pid % 1000) as u16;
		let local_udp_addr = SocketAddr::new(local_ip, udp_port);
		
		let msg = NetworkMessage::JoinRequest(current_pid);
		writeln!(stream, "{}", serialize_message(&msg).map_err(|e| format!("Serialize error: {}", e))?)
			.map_err(|e| format!("Write error: {}", e))?;
		stream.flush().map_err(|e| format!("Flush error: {}", e))?;
		
		let mut reader = BufReader::new(&mut stream);
		let mut line = String::new();
		reader.read_line(&mut line).map_err(|e| format!("Read error: {}", e))?;
		
		let host_addr = match deserialize_message(&line.trim()).map_err(|e| format!("Parse error: {}", e))? {
			NetworkMessage::JoinResponse(addr) => addr,
			_ => return Err("Wrong response type".into()),
		};
		
		let msg = NetworkMessage::PeerAddress(local_udp_addr);
		writeln!(stream, "{}", serialize_message(&msg).map_err(|e| format!("Serialize error: {}", e))?)
			.map_err(|e| format!("Write error: {}", e))?;
		stream.flush().map_err(|e| format!("Flush error: {}", e))?;
		
		Ok((local_udp_addr, host_addr))
	}

	pub fn check_pending_connections(&mut self) -> Result<bool, String> {
		let mut completed = false;
		let mut i = 0;
		
		while i < self.pending_connections.len() {
			if self.pending_connections[i].handle.is_finished() {
				let pending = self.pending_connections.remove(i);
				match pending.handle.join() {
					Ok(Ok((local, remote))) => {
						self.local_udp_addr = Some(local);
						self.remote_udp_addr = Some(remote);
						completed = true;
					}
					Ok(Err(e)) if !e.contains("No data available yet") && !e.contains("Waiting for peer address") => {
						let msg = format!("Connection to {} failed: {}", pending.peer_addr, e);
						self.status = NetworkStatus::Error(msg.clone());
						self.push_event(NetworkEvent::Error(msg));
					}
					Err(_) => {
						let msg = format!("Thread panicked for {}", pending.peer_addr);
						self.status = NetworkStatus::Error(msg.clone());
						self.push_event(NetworkEvent::Error(msg));
					}
					_ => {}
				}
			} else {
				i += 1;
			}
		}
		Ok(completed)
	}

	fn broadcast_listener_thread(local_ip: String) {
		let socket = match UdpSocket::bind(format!("0.0.0.0:{}", DISCOVERY_PORT)) {
			Ok(s) => s,
			Err(e) => { println!("Bind error: {}", e); return; }
		};

		let mut buf = [0; 1024];
		loop {
			match socket.recv_from(&mut buf) {
				Ok((size, sender)) => {
					if let Ok(NetworkMessage::DiscoveryRequest) = deserialize_from_bytes(&buf[..size]) {
						let res = NetworkMessage::DiscoveryResponse {
							ip: local_ip.clone(),
							port: TCP_PORT,
						};
						if let Ok(data) = serialize_to_bytes(&res) {
							let _ = socket.send_to(&data, sender);
						}
					}
				}
				Err(e) => println!("Recv error: {}", e),
			}
		}
	}

	fn discover_hosts_broadcast(timeout_ms: u64) -> DiscoveryResult {
		let mut res = DiscoveryResult {
			hosts: Vec::new(),
			debug_info: vec!["Starting broadcast discovery".into()],
			errors: Vec::new(),
		};
		
		let local_ip = types::get_local_ip_string();        
		let socket = match UdpSocket::bind("0.0.0.0:0") {
			Ok(s) => s,
			Err(e) => { res.errors.push(format!("Socket error: {}", e)); return res; }
		};
		
		socket.set_broadcast(true).ok();
		socket.set_read_timeout(Some(Duration::from_millis(timeout_ms))).ok();
				
		let broadcast_addr = format!("{}:{}", types::get_broadcast_address(&local_ip), DISCOVERY_PORT);
		if let Ok(data) = serialize_to_bytes(&NetworkMessage::DiscoveryRequest) {
			if socket.send_to(&data, &broadcast_addr).is_err() {
				res.errors.push("Broadcast failed".into());
				return res;
			}
		} else {
			res.errors.push("Failed to serialize discovery request".into());
			return res;
		}
		
		let start = Instant::now();
		let mut buf = [0; 1024];
		
		while start.elapsed() < Duration::from_millis(timeout_ms) {
			match socket.recv_from(&mut buf) {
				Ok((size, _)) => {
					if let Ok(NetworkMessage::DiscoveryResponse { ip, port }) = deserialize_from_bytes(&buf[..size]) {
						let address = match ip.contains(':') {
							true => ip.parse::<SocketAddr>(),
							false => format!("{}:{}", ip, port).parse::<SocketAddr>(),
						}.map_err(|e| format!("Invalid address format: {}", e));
						
						if let Ok(address) = address {
							let world_name = Self::get_world_name_from_host(address, &mut res.debug_info, &mut res.errors)
								.unwrap_or_else(|| "Unknown".to_string());
							
							res.debug_info.push(format!("Found host: {}:{} ({:?})", ip, port, address));
							res.hosts.push(HostInfo {
								pid: port as u32,
								address,
								world_name,
							});
						}
					}
				}
				Err(e) if e.kind() != io::ErrorKind::TimedOut => res.errors.push(format!("Recv error: {}", e)),
				_ => {}
			}
		}
		
		res.debug_info.push(format!("Found {} hosts", res.hosts.len()));
		res
	}
	
	pub fn get_world_name_from_host(addr: SocketAddr, debug: &mut Vec<String>, errors: &mut Vec<String>) -> Option<String> {
		let mut stream = match TcpStream::connect_timeout(&addr, Duration::from_millis(2000)) {
			Ok(s) => s,
			Err(e) => { errors.push(format!("Connection failed to {}: {}", addr, e)); return None; }
		};
		
		if let Err(e) = stream.set_read_timeout(Some(Duration::from_millis(2000))) {
			errors.push(format!("Failed to set timeout for {}: {}", addr, e)); 
			return None;
		}
		
		let msg = NetworkMessage::WorldInfoRequest;
		if let Err(e) = writeln!(stream, "{}", serialize_message(&msg).ok()?) {
			errors.push(format!("Failed to send request to {}: {}", addr, e)); 
			return None;
		}
		
		if let Err(e) = stream.flush() { 
			errors.push(format!("Failed to flush to {}: {}", addr, e)); 
			return None; 
		}
		
		let mut reader = BufReader::new(&stream);
		let mut line = String::new();
		if let Err(e) = reader.read_line(&mut line) { 
			errors.push(format!("Failed to read response from {}: {}", addr, e)); 
			return None; 
		}
		
		let trimmed = line.trim();
		if trimmed.is_empty() { 
			errors.push(format!("Empty response from {}", addr)); 
			return None; 
		}
		
		match deserialize_message(trimmed) {
			Ok(NetworkMessage::WorldInfoResponse(name)) => {
				debug.push(format!("Got world '{}' from {}", name, addr));
				Some(name)
			}
			Ok(_) => { errors.push(format!("Unexpected message type from {}", addr)); None }
			Err(e) => { 
				errors.push(format!("Parse error from {}: {} (response: '{}')", addr, e, trimmed)); 
				None 
			}
		}
	}
}

#[inline]
pub fn discover_hosts(timeout_ms: u64) -> Result<String, String> {
	let system = api::get_ptr().ok_or("Network system not initialized")?;
	system.discovery_thread = Some(thread::spawn(move || 
		NetworkSystem::discover_hosts_broadcast(timeout_ms)
	));
	Ok(format!("Broadcast discovery started with timeout: {}ms", timeout_ms))
}

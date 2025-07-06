
use steamworks::{Client, FriendFlags, PersonaStateChange, LobbyKey, SteamId, StringFilterKind, StringFilter, LobbyId, LobbyType, LobbyChatUpdate, LobbyEnter};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicPtr, AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::ptr;
use serde::{Deserialize, Serialize};

// Static system pointer for global access
static STEAM_NETWORK_PTR: AtomicPtr<SteamNetworkSystem> = AtomicPtr::new(ptr::null_mut());
static STEAM_NETWORK_INITIALIZED: AtomicBool = AtomicBool::new(false);

// Network events that match your old API
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    Connected(SteamId),
    Disconnected(SteamId),
    GameStateUpdate(i32),
    Error(String),
    Synchronizing,
    Ready,
    HostsDiscovered(Vec<HostInfo>),
    DiscoveryComplete(DiscoveryResult),
    LobbyJoined(LobbyId),
    PlayerJoined(SteamId, String),
    PlayerLeft(SteamId, String),
    MessageReceived(SteamId, String),
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
    pub steam_id: SteamId,
    pub lobby_id: LobbyId,
    pub world_name: String,
    pub player_count: u32,
    pub max_players: u32,
}

#[derive(Debug, Clone)]
pub struct DiscoveryResult {
    pub hosts: Vec<HostInfo>,
    pub debug_info: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum GameMessage {
    JoinRequest,
    JoinAccepted,
    JoinRejected(String),
    GameState(i32),
    PlayerInput(i32),
    ChatMessage(String),
    WorldSync(String),
    Disconnect,
}

pub struct SteamNetworkSystem {
    pub client: Client,
    pub status: NetworkStatus,
    pub is_host: bool,
    pub current_lobby: Option<LobbyId>,
    pub connected_peers: HashMap<SteamId, String>,
    pub event_queue: VecDeque<NetworkEvent>,
    pub frame_count: u32,
    pub game_state: i32,
    pub last_frame_time: Instant,
    pub world_name: String,
    pub max_players: u32,
    pub discovery_thread: Option<thread::JoinHandle<DiscoveryResult>>,
    pub discovered_hosts: Arc<Mutex<Vec<HostInfo>>>,
    pub consecutive_errors: u32,
    pub callbacks: CallbackHandlers,
}

pub struct CallbackHandlers {
    pub lobby_enter: Option<steamworks::CallbackHandle>,
    pub lobby_chat_update: Option<steamworks::CallbackHandle>,
    pub persona_state_change: Option<steamworks::CallbackHandle>,
}

impl SteamNetworkSystem {
    pub fn new(world_name: String, max_players: u32) -> Result<Self, Box<dyn std::error::Error>> {
        let client = Client::init()?;
        
        Ok(SteamNetworkSystem {
            client,
            status: NetworkStatus::Idle,
            is_host: false,
            current_lobby: None,
            connected_peers: HashMap::new(),
            event_queue: VecDeque::new(),
            frame_count: 0,
            game_state: 0,
            last_frame_time: Instant::now(),
            world_name,
            max_players,
            discovery_thread: None,
            discovered_hosts: Arc::new(Mutex::new(Vec::new())),
            consecutive_errors: 0,
            callbacks: CallbackHandlers {
                lobby_enter: None,
                lobby_chat_update: None,
                persona_state_change: None,
            },
        })
    }

    pub fn setup_callbacks(&mut self) {
        // Lobby enter callback
        self.callbacks.lobby_enter = Some(self.client.register_callback(move |enter: LobbyEnter| {
            if let Some(system) = get_ptr() {
                // Access the lobby field directly (not as a method)
                let lobby_id = enter.lobby;
                system.current_lobby = Some(lobby_id);
                system.status = NetworkStatus::Connected;
                system.push_event(NetworkEvent::LobbyJoined(lobby_id));
            }
        }));

        // Lobby chat update callback
        self.callbacks.lobby_chat_update = Some(self.client.register_callback(move |update: LobbyChatUpdate| {
            if let Some(system) = get_ptr() {
                let user_id = update.user_changed;
                let user_name = system.get_friend_name(user_id);
                
                // Access the state_change field directly
                match update.member_state_change {
                    steamworks::ChatMemberStateChange::Entered => {
                        system.connected_peers.insert(user_id, user_name.clone());
                        system.push_event(NetworkEvent::PlayerJoined(user_id, user_name));
                    }
                    steamworks::ChatMemberStateChange::Left | 
                    steamworks::ChatMemberStateChange::Disconnected |
                    steamworks::ChatMemberStateChange::Kicked |
                    steamworks::ChatMemberStateChange::Banned => {
                        system.connected_peers.remove(&user_id);
                        system.push_event(NetworkEvent::PlayerLeft(user_id, user_name));
                    }
                }
            }
        }));

        // Persona state change callback
        self.callbacks.persona_state_change = Some(self.client.register_callback(|change: PersonaStateChange| {
            println!("Persona state changed: {:?}", change.steam_id);
        }));
    }

    pub fn push_event(&mut self, event: NetworkEvent) {
        self.event_queue.push_back(event);
        // Keep queue size reasonable
        if self.event_queue.len() > 100 {
            self.event_queue.pop_front();
        }
    }

    pub fn start_as_host(&mut self) -> Result<String, String> {
        self.is_host = true;
        self.status = NetworkStatus::Discovering;
        
        let matchmaking = self.client.matchmaking();
        
        // Create lobby with callback
        matchmaking.create_lobby(LobbyType::FriendsOnly, self.max_players, |result| {
            match result {
                Ok(lobby_id) => {
                    if let Some(system) = get_ptr() {
                        system.current_lobby = Some(lobby_id);
                        
                        // Set lobby metadata
                        let matchmaking = system.client.matchmaking();
                        matchmaking.set_lobby_data(lobby_id, "world_name", &system.world_name);
                        matchmaking.set_lobby_data(lobby_id, "game_mode", "multiplayer");
                        matchmaking.set_lobby_data(lobby_id, "version", "1.0");
                        
                        system.push_event(NetworkEvent::Ready);
                    }
                }
                Err(e) => {
                    if let Some(system) = get_ptr() {
                        system.push_event(NetworkEvent::Error(format!("Failed to create lobby: {:?}", e)));
                    }
                }
            }
        });
        
        Ok("Creating lobby as host...".to_string())
    }

    pub fn start_discovery(&mut self) -> Result<String, String> {
        self.status = NetworkStatus::Discovering;
        
        let matchmaking = self.client.matchmaking();
        
        // Correct way to add string filter
        matchmaking.add_request_lobby_list_string_filter(
            StringFilter( LobbyKey::new("game_mode"), "multiplayer", StringFilterKind::Equal )
        );
        
        // Request lobby list with callback
        matchmaking.request_lobby_list(|result| {
            if let Some(system) = get_ptr() {
                match result {
                    Ok(lobby_list) => {
                        let mut hosts = Vec::new();
                        
                        // Iterate directly over the Vec<LobbyId>
                        for lobby_id in lobby_list {
                            let matchmaking = system.client.matchmaking();
                            let world_name = matchmaking.lobby_data(lobby_id, "world_name")
                                .unwrap_or_else(|| "Unknown".to_string());
                            let owner = matchmaking.lobby_owner(lobby_id);
                            let member_count = matchmaking.lobby_member_count(lobby_id);
                            let member_limit = matchmaking.lobby_member_limit(lobby_id)
                                .unwrap_or(0); // Default to 0 if None
                            
                            hosts.push(HostInfo {
                                steam_id: owner,
                                lobby_id,
                                world_name,
                                player_count: member_count as u32,
                                max_players: member_limit as u32,
                            });
                        }
                        
                        system.push_event(NetworkEvent::HostsDiscovered(hosts));
                    }
                    Err(e) => {
                        system.push_event(NetworkEvent::Error(format!("Discovery failed: {:?}", e)));
                    }
                }
            }
        });
        
        Ok("Started lobby discovery".to_string())
    }

    pub fn connect_to_host(&mut self, lobby_id: LobbyId) -> Result<String, String> {
        self.status = NetworkStatus::Connecting;
        
        let matchmaking = self.client.matchmaking();
        
        // Join lobby with callback
        matchmaking.join_lobby(lobby_id, |result| {
            if let Some(system) = get_ptr() {
                match result {
                    Ok(joined_lobby_id) => {
                        system.current_lobby = Some(joined_lobby_id);
                        system.status = NetworkStatus::Connected;
                        system.push_event(NetworkEvent::LobbyJoined(joined_lobby_id));
                    }
                    Err(_) => {
                        system.push_event(NetworkEvent::Error("Failed to join lobby".to_string()));
                    }
                }
            }
        });
        
        Ok(format!("Attempting to join lobby: {:?}", lobby_id))
    }

    pub fn send_message(&self, target: SteamId, message: &str) -> Result<(), String> {
        let game_msg = GameMessage::ChatMessage(message.to_string());
        self.send_game_message(target, &game_msg)
    }

    pub fn send_game_message(&self, target: SteamId, message: &GameMessage) -> Result<(), String> {
        let networking = self.client.networking();
        
        match serde_json::to_vec(message) {
            Ok(data) => {
                let success = networking.send_p2p_packet(
                    target,
                    steamworks::SendType::Reliable,
                    &data,
                );
                
                if success {
                    Ok(())
                } else {
                    Err(format!("Failed to send message to {:?}", target))
                }
            }
            Err(e) => Err(format!("Failed to serialize message: {}", e))
        }
    }

    pub fn broadcast_message(&self, message: &str) -> Result<(), String> {
        let game_msg = GameMessage::ChatMessage(message.to_string());
        self.broadcast_game_message(&game_msg)
    }

    pub fn broadcast_game_message(&self, message: &GameMessage) -> Result<(), String> {
        for peer_id in self.connected_peers.keys() {
            if let Err(e) = self.send_game_message(*peer_id, message) {
                println!("Failed to send to {:?}: {}", peer_id, e);
            }
        }
        Ok(())
    }

    pub fn handle_incoming_packets(&mut self) -> Result<(), String> {
        let networking = self.client.networking();
        
        while let Some(size) = networking.is_p2p_packet_available() {
            let mut buffer = vec![0u8; size];
            
            if let Some((sender_id, actual_size)) = networking.read_p2p_packet(&mut buffer) {
                buffer.truncate(actual_size);
                self.handle_packet(sender_id, &buffer)?;
            }
        }
        Ok(())
    }

    pub fn handle_packet(&mut self, sender: SteamId, data: &[u8]) -> Result<(), String> {
        match serde_json::from_slice::<GameMessage>(data) {
            Ok(message) => {
                match message {
                    GameMessage::JoinRequest => {
                        if self.is_host {
                            self.handle_join_request(sender)?;
                        }
                    }
                    GameMessage::JoinAccepted => {
                        if !self.is_host {
                            self.status = NetworkStatus::Connected;
                            self.push_event(NetworkEvent::Connected(sender));
                        }
                    }
                    GameMessage::JoinRejected(reason) => {
                        self.push_event(NetworkEvent::Error(format!("Join rejected: {}", reason)));
                    }
                    GameMessage::GameState(state) => {
                        self.game_state = state;
                        self.push_event(NetworkEvent::GameStateUpdate(state));
                    }
                    GameMessage::PlayerInput(input) => {
                        // Handle player input
                        println!("Received input {} from {:?}", input, sender);
                    }
                    GameMessage::ChatMessage(msg) => {
                        let sender_name = self.get_friend_name(sender);
                        // Clone the message before moving it
                        self.push_event(NetworkEvent::MessageReceived(sender, msg.clone()));
                        println!("[CHAT] {}: {}", sender_name, msg);
                    }
                    GameMessage::WorldSync(world_data) => {
                        // Handle world synchronization
                        println!("Received world sync from {:?}: {}", sender, world_data);
                    }
                    GameMessage::Disconnect => {
                        self.handle_disconnect(sender);
                    }
                }
                Ok(())
            }
            Err(_e) => {
                // Try to handle as raw message for compatibility
                let message = String::from_utf8_lossy(data);
                println!("Received raw message from {:?}: {}", sender, message);
                Ok(())
            }
        }
    }

    pub fn handle_join_request(&mut self, sender: SteamId) -> Result<(), String> {
        if self.connected_peers.len() >= self.max_players as usize {
            self.send_game_message(sender, &GameMessage::JoinRejected("Lobby full".to_string()))?;
            return Ok(());
        }

        let sender_name = self.get_friend_name(sender);
        self.connected_peers.insert(sender, sender_name.clone());
        self.send_game_message(sender, &GameMessage::JoinAccepted)?;
        
        // Send world sync
        self.send_game_message(sender, &GameMessage::WorldSync(self.world_name.clone()))?;
        
        println!("Player {} joined the game", sender_name);
        Ok(())
    }

    pub fn handle_disconnect(&mut self, sender: SteamId) {
        if let Some(name) = self.connected_peers.remove(&sender) {
            self.push_event(NetworkEvent::PlayerLeft(sender, name.clone()));
            println!("Player {} disconnected", name);
        }
    }

    pub fn get_friend_name(&self, steam_id: SteamId) -> String {
        let friends = self.client.friends();
        let friend_list = friends.get_friends(FriendFlags::IMMEDIATE);
        
        for friend in friend_list {
            if friend.id() == steam_id {
                return friend.name();
            }
        }
        
        format!("Player_{}", steam_id.raw())
    }

    pub fn get_my_steam_id(&self) -> SteamId {
        self.client.user().steam_id()
    }

    pub fn update_game_session(&mut self) {
        // Simulate game logic
        self.frame_count += 1;
        
        if self.frame_count % 60 == 0 {
            self.game_state += 1;
            if self.is_host {
                let _ = self.broadcast_game_message(&GameMessage::GameState(self.game_state));
            }
        }
        
        if self.frame_count % 300 == 0 {
            self.push_event(NetworkEvent::GameStateUpdate(self.game_state));
        }
    }

    pub fn start_discovery_thread(&mut self, timeout_ms: u64) {
        let discovered_hosts = Arc::clone(&self.discovered_hosts);
        let handle = thread::spawn(move || {
            discover_lobbies_threaded(timeout_ms, discovered_hosts)
        });
        
        self.discovery_thread = Some(handle);
    }

    pub fn check_discovery_complete(&mut self) -> bool {
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
                self.discovery_thread = Some(handle);
            }
        }
        false
    }

    pub fn shutdown(&mut self) {
        println!("Shutting down Steam networking...");
        
        // Send disconnect to all peers
        let _ = self.broadcast_game_message(&GameMessage::Disconnect);
        thread::sleep(Duration::from_millis(100));
        
        // Close P2P sessions
        let networking = self.client.networking();
        for peer_id in self.connected_peers.keys() {
            networking.close_p2p_session(*peer_id);
        }
        
        // Leave lobby if in one
        if let Some(lobby_id) = self.current_lobby {
            let matchmaking = self.client.matchmaking();
            matchmaking.leave_lobby(lobby_id);
        }
        
        self.connected_peers.clear();
        self.current_lobby = None;
        self.status = NetworkStatus::Idle;
    }
}

// Helper function to safely access the SteamNetworkSystem pointer
#[inline]
pub fn get_ptr() -> Option<&'static mut SteamNetworkSystem> {
    let system_ptr = STEAM_NETWORK_PTR.load(Ordering::Acquire);
    if system_ptr.is_null() {
        None
    } else {
        unsafe { Some(&mut *system_ptr) }
    }
}

// Public API functions that match your old networking system

/// Initialize the Steam network system
#[inline]
pub fn init_network(world_name: String, max_players: u32) -> Result<String, String> {
    if STEAM_NETWORK_INITIALIZED.load(Ordering::Acquire) {
        return Err("Steam network system already initialized".to_string());
    }
    
    match SteamNetworkSystem::new(world_name.clone(), max_players) {
        Ok(mut system) => {
            system.setup_callbacks();
            
            let old_ptr = STEAM_NETWORK_PTR.swap(Box::into_raw(Box::new(system)), Ordering::AcqRel);
            if !old_ptr.is_null() {
                unsafe { let _ = Box::from_raw(old_ptr); }
            }
            
            STEAM_NETWORK_INITIALIZED.store(true, Ordering::Release);
            Ok(format!("Steam network initialized for world: {}", world_name))
        }
        Err(e) => Err(format!("Failed to initialize Steam: {}", e))
    }
}

#[inline]
pub fn is_running() -> bool {
    STEAM_NETWORK_INITIALIZED.load(Ordering::Acquire)
}

#[inline]
pub fn is_host() -> Result<bool, String> {
    if let Some(system) = get_ptr() {
        Ok(system.is_host)
    } else {
        Err("Steam network system not initialized".to_string())
    }
}

#[inline]
pub fn begin_online_search() -> Result<String, String> {
    cleanup_network();
    match init_network("none".to_string(),0) {
        Ok(_) => {},
        Err(e) => {return Err(e);},
    };
    if let Some(system) = get_ptr() {
        system.start_discovery_thread(200);
        Ok("Started Steam lobby discovery".to_string())
    } else {
        Err("Steam network system not initialized".to_string())
    }
}

#[inline]
pub fn begin_online_giveaway(world_name: String) -> Result<String, String> {
    cleanup_network();
    match init_network(world_name,2) {
        Ok(_) => {},
        Err(e) => {return Err(e);},
    };
    if let Some(system) = get_ptr() {
        system.start_as_host()
    } else {
        Err("Steam network system not initialized".to_string())
    }
}

#[inline]
pub fn start_host() -> Result<String, String> {
    if let Some(system) = get_ptr() {
        system.start_as_host()
    } else {
        Err("Steam network system not initialized".to_string())
    }
}

#[inline]
pub fn cleanup_network() {
    if let Some(system) = get_ptr() {
        system.shutdown();
    }
    
    let old_ptr = STEAM_NETWORK_PTR.swap(ptr::null_mut(), Ordering::AcqRel);
    if !old_ptr.is_null() {
        unsafe { let _ = Box::from_raw(old_ptr); }
    }
    STEAM_NETWORK_INITIALIZED.store(false, Ordering::Release);
}

#[inline]
pub fn update_network() {
    if let Some(system) = get_ptr() {
        // Run Steam callbacks
        system.client.run_callbacks();
        
        // Handle incoming packets
        if let Err(e) = system.handle_incoming_packets() {
            system.push_event(NetworkEvent::Error(format!("Packet handling error: {}", e)));
        }
        
        // Check discovery completion
        system.check_discovery_complete();
        
        // Limit update rate
        let now = Instant::now();
        if now.duration_since(system.last_frame_time) < Duration::from_millis(16) {
            return;
        }
        system.last_frame_time = now;
        
        // Update game session
        if matches!(system.status, NetworkStatus::Connected | NetworkStatus::InGame) {
            system.update_game_session();
        }
    }

    // Handle events
    while let Some(event) = pop_network_event() {
        match event {
            NetworkEvent::PlayerJoined(steam_id, name) => {
                println!("Player joined: {} ({})", name, steam_id.raw());
            }
            NetworkEvent::MessageReceived(steam_id, message) => {
                println!("Message from {}: {}", steam_id.raw(), message);
            }
            NetworkEvent::Error(error) => {
                println!("Network error: {}", error);
            }
            _ => println!("Network event: {:?}", event),
        }
    }
}

#[inline]
pub fn get_discovered_hosts() -> Vec<HostInfo> {
    if let Some(system) = get_ptr() {
        system.discovered_hosts.lock().unwrap().clone()
    } else {
        Vec::new()
    }
}

#[inline]
pub fn connect_to_host(lobby_id: u64) -> Result<String, String> {
    if let Some(system) = get_ptr() {
        let lobby = LobbyId::from_raw(lobby_id);
        system.connect_to_host(lobby)
    } else {
        Err("Steam network system not initialized".to_string())
    }
}

#[inline]
pub fn get_network_status() -> NetworkStatus {
    if let Some(system) = get_ptr() {
        system.status.clone()
    } else {
        NetworkStatus::Error("Steam network system not initialized".to_string())
    }
}

#[inline]
pub fn pop_network_event() -> Option<NetworkEvent> {
    if let Some(system) = get_ptr() {
        system.event_queue.pop_front()
    } else {
        None
    }
}

#[inline]
pub fn get_game_state() -> i32 {
    if let Some(system) = get_ptr() {
        system.game_state
    } else {
        0
    }
}

#[inline]
pub fn get_frame_count() -> u32 {
    if let Some(system) = get_ptr() {
        system.frame_count
    } else {
        0
    }
}

#[inline]
pub fn broadcast_message(message: &str) -> Result<(), String> {
    if let Some(system) = get_ptr() {
        system.broadcast_message(message)
    } else {
        Err("Steam network system not initialized".to_string())
    }
}

#[inline]
pub fn get_my_steam_id() -> Option<SteamId> {
    if let Some(system) = get_ptr() {
        Some(system.get_my_steam_id())
    } else {
        None
    }
}

// Discovery function that runs in a separate thread
fn discover_lobbies_threaded(timeout_ms: u64, discovered_hosts: Arc<Mutex<Vec<HostInfo>>>) -> DiscoveryResult {
    let hosts = Vec::new();
    let mut debug_info = Vec::new();
    let errors = Vec::new();
    
    debug_info.push("Starting Steam lobby discovery".to_string());
    
    // In a real implementation, you would use the Steam callback system
    // For now, we'll simulate finding some lobbies
    thread::sleep(Duration::from_millis(timeout_ms));
    
    // Update the shared host list
    if let Ok(mut shared_hosts) = discovered_hosts.lock() {
        shared_hosts.clear();
        shared_hosts.extend(hosts.clone());
    }
    
    debug_info.push(format!("Discovery complete. Found {} lobbies", hosts.len()));
    
    DiscoveryResult {
        hosts,
        debug_info,
        errors,
    }
}
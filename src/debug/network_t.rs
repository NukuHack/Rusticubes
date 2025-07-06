

#[cfg(test)]
use crate::ext::network_discovery;
#[cfg(test)]
use crate::game_state;
#[cfg(test)]
use std::thread;
#[cfg(test)]
use std::time::Duration;
#[cfg(test)]
use crate::ext::network_api;
#[test] #[ignore]
pub fn network_test() {
    println!("=== STARTING NETWORK TEST ===");
    
    // Step 1: Start the search
    println!("\n1. Starting online search...");
    match network_api::begin_online_search() {
        Ok(o) => println!("✓ Search started: {}", o),
        Err(e) => println!("✗ Search failed: {}", e),
    }
        
    // Step 2: Test direct connection first (this should work if host is running)
    println!("\n2. Testing direct connection to host...");
    match network_discovery::test_host_connection() {
        Ok(o) => println!("✓ Direct connection test: {}", o),
        Err(e) => println!("✗ Direct connection test: {}", e),
    }
    
    // Step 3: Wait for discovery with detailed monitoring
    println!("\n3. Waiting for discovery to complete...");
    thread::sleep(Duration::from_millis(1000));
    // Update the network system
    network_api::update_network();
        
    // Check for events
    while let Some(event) = network_api::pop_network_event() {
        println!("   EVENT: {:?}", event);
    }
    
    // Check discovered hosts
    let hosts = network_api::get_discovered_hosts();
    if !hosts.is_empty() {
        println!("   Found {} hosts!", hosts.len());
        for (i, host) in hosts.iter().enumerate() {
            println!("     Host {}: PID={}, Addr={}, World='{}'", 
                     i + 1, host.pid, host.address, host.world_name);
        }
    }
    
    
    // Step 4: Final results
    println!("\n4. Final results:");
    let final_hosts = network_api::get_discovered_hosts();
    println!("   Total hosts found: {}", final_hosts.len());
    
    if final_hosts.is_empty() {
        println!("   ⚠ No hosts discovered!");
        
        // Additional debugging
        println!("\n4,5. Additional debugging:");
        println!("   Network status: {:?}", network_api::get_network_status());
        println!("   Is network running: {}", network_api::is_running());
        
        // Try one more direct test
        println!("\n   Retrying direct connection test...");
        match network_discovery::test_host_connection() {
            Ok(o) => println!("   ✓ Retry successful: {}", o),
            Err(e) => println!("   ✗ Retry failed: {}", e),
        }
    } else {
        println!("   ✓ Discovery successful!");
        for host in final_hosts {
            println!("     - PID: {}, Address: {}, World: '{}'", 
                     host.pid, host.address, host.world_name);
        }
    }
    
    // Step 5: Cleanup
    println!("\n5. Cleaning up...");
    network_api::cleanup_network();
    println!("   ✓ Network cleaned up");
    
    println!("\n=== NETWORK TEST COMPLETE ===");
}

// Additional test to run a host for testing
#[test] #[ignore]
pub fn network_host_test() {
    println!("=== STARTING HOST TEST ===");

    game_state::start_world("test_world");
    
    // Start as host
    match network_api::begin_online_giveaway() {
        Ok(o) => println!("✓ Host started: {}", o),
        Err(e) => {
            println!("✗ Host failed: {}", e);
            return;
        }
    }
    
    // Keep the host running for a bit
    println!("Host running for 10 seconds...");
    for i in 0..100 {
        network_api::update_network();
        
        // Check for events
        while let Some(event) = network_api::pop_network_event() {
            println!("HOST EVENT: {:?}", event);
        }
        
        if i % 10 == 0 {
            println!("Host tick {}/100, Status: {:?}", i, network_api::get_network_status());
        }
        
        thread::sleep(Duration::from_millis(100));
    }
    
    network_api::cleanup_network();
    println!("=== HOST TEST COMPLETE ===");
}

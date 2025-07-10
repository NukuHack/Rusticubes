

#[cfg(test)]
use crate::network::discovery;
#[cfg(test)]
use crate::network::api;
#[cfg(test)]
use crate::game::state;
#[cfg(test)]
use std::thread;
#[cfg(test)]
use std::time::Duration;
#[test] #[ignore]
pub fn network_test() {
    println!("=== STARTING NETWORK TEST ===");
    
    // Step 1: Start the search
    println!("\n1. Starting online search...");
    match api::begin_online_search() {
        Ok(o) => println!("✓ Search started: {}", o),
        Err(e) => println!("✗ Search failed: {}", e),
    }
        
    // Step 2: Test direct connection first (this should work if host is running)
    println!("\n2. Testing direct connection to host...");
    match discovery::test_host_connection() {
        Ok(o) => println!("✓ Direct connection test: {}", o),
        Err(e) => println!("✗ Direct connection test: {}", e),
    }
    
    // Step 3: Wait for discovery with detailed monitoring
    println!("\n3. Waiting for discovery to complete...");
    thread::sleep(Duration::from_millis(1000));
    // Update the network system
    api::update_network();
    
    // Check discovered hosts
    let hosts = api::get_discovered_hosts();
    if !hosts.is_empty() {
        println!("   Found {} hosts!", hosts.len());
        for (i, host) in hosts.iter().enumerate() {
            println!("     Host {}: PID={}, Addr={}, World='{}'", 
                     i + 1, host.pid, host.address, host.world_name);
        }
    }
    
    
    // Step 4: Final results
    println!("\n4. Final results:");
    let final_hosts = api::get_discovered_hosts();
    println!("   Total hosts found: {}", final_hosts.len());
    
    if final_hosts.is_empty() {
        println!("   ⚠ No hosts discovered!");
        
        // Additional debugging
        println!("\n4,5. Additional debugging:");
        println!("   Is network running: {}", api::is_running());
        
        // Try one more direct test
        println!("\n   Retrying direct connection test...");
        match discovery::test_host_connection() {
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
    api::cleanup_network();
    println!("   ✓ Network cleaned up");
    
    println!("\n=== NETWORK TEST COMPLETE ===");
}

// Additional test to run a host for testing
#[test] #[ignore]
pub fn network_host_test() {
    println!("=== STARTING HOST TEST ===");

    state::start_world("test_world");
    
    // Start as host
    match api::begin_online_giveaway() {
        Ok(o) => println!("✓ Host started: {}", o),
        Err(e) => {
            println!("✗ Host failed: {}", e);
            return;
        }
    }
    
    // Keep the host running for a bit
    println!("Host running for 10 seconds...");
    for _i in 0..100 {
        api::update_network();
        
        thread::sleep(Duration::from_millis(100));
    }
    
    api::cleanup_network();
    println!("=== HOST TEST COMPLETE ===");
}

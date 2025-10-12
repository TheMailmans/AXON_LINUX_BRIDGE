// Simple test client to diagnose GetFrame RPC
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{'='*60}");
    println!("🔍 BRIDGE GetFrame DIAGNOSTIC TEST");
    println!("{'='*60}\n");
    
    // Import the generated proto code
    mod agent {
        tonic::include_proto!("agent");
    }
    
    use agent::desktop_agent_client::DesktopAgentClient;
    use agent::{HeartbeatRequest, ConnectRequest, GetFrameRequest};
    
    let bridge_addr = "http://localhost:50051";
    println!("📡 Connecting to bridge at {}...", bridge_addr);
    
    let mut client = DesktopAgentClient::connect(bridge_addr).await?;
    println!("✅ Connected!\n");
    
    // Test 1: Heartbeat
    println!("1️⃣ Testing Heartbeat RPC...");
    let start = Instant::now();
    let response = client.heartbeat(HeartbeatRequest {
        agent_id: "test-diagnostic".to_string(),
    }).await?;
    let elapsed = start.elapsed();
    println!("   ✅ Heartbeat OK ({:.2}s)", elapsed.as_secs_f64());
    println!("   Status: {}, Timestamp: {}\n", response.get_ref().status, response.get_ref().server_timestamp);
    
    // Test 2: RegisterAgent
    println!("2️⃣ Registering test agent...");
    let start = Instant::now();
    let response = client.register_agent(ConnectRequest {
        session_id: "test-diagnostic-session".to_string(),
        hub_url: "http://localhost:4545".to_string(),
    }).await?;
    let elapsed = start.elapsed();
    let agent_id = response.get_ref().agent_id.clone();
    println!("   ✅ RegisterAgent OK ({:.2}s)", elapsed.as_secs_f64());
    println!("   Agent ID: {}\n", agent_id);
    
    // Test 3: GetFrame (THE CRITICAL TEST)
    println!("3️⃣ Testing GetFrame RPC (timeout=10s)...");
    println!("   ⏱️  Starting GetFrame call...");
    let start = Instant::now();
    
    match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        client.get_frame(GetFrameRequest { agent_id: agent_id.clone() })
    ).await {
        Ok(Ok(response)) => {
            let elapsed = start.elapsed();
            let frame = response.get_ref().frame.as_ref().unwrap();
            println!("   ✅ GetFrame SUCCESS! ({:.2}s)", elapsed.as_secs_f64());
            println!("   Frame: {}x{}, {} bytes", frame.width, frame.height, frame.data.len());
            println!("   Format: {}, Sequence: {}", frame.format, frame.sequence_number);
            
            // Save to file
            std::fs::write("/tmp/bridge_test_frame.png", &frame.data)?;
            println!("   💾 Saved frame to: /tmp/bridge_test_frame.png\n");
            
            println!("{'='*60}");
            println!("✅ DIAGNOSIS: Bridge GetFrame is WORKING");
            println!("   → Issue is likely in the HUB/SERVER side");
            println!("{'='*60}\n");
        }
        Ok(Err(e)) => {
            let elapsed = start.elapsed();
            println!("   ❌ GetFrame FAILED: {} (after {:.2}s)\n", e, elapsed.as_secs_f64());
            
            println!("{'='*60}");
            println!("❌ DIAGNOSIS: Bridge GetFrame RPC ERROR");
            println!("   → Error: {}", e);
            println!("{'='*60}\n");
        }
        Err(_) => {
            let elapsed = start.elapsed();
            println!("   ❌ GetFrame TIMEOUT after {:.2}s\n", elapsed.as_secs_f64());
            
            println!("{'='*60}");
            println!("❌ DIAGNOSIS: Bridge GetFrame is HANGING");
            println!("   → Issue is in the BRIDGE implementation");
            println!("   → Likely causes:");
            println!("      • X11 capture hanging in LinuxCapturer");
            println!("      • Native capture blocking indefinitely");
            println!("      • Resource deadlock in bridge");
            println!("{'='*60}\n");
        }
    }
    
    Ok(())
}

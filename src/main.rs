use anyhow::Result;
use tracing::{info, error};
use tracing_subscriber;

mod agent;
mod audio;
mod capture;
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod desktop_apps;
mod grpc_service;
mod input;
mod platform;
mod proto_gen;
mod streaming;
mod video;
mod a11y;

use agent::Agent;
use grpc_service::DesktopAgentService;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();

    info!("Starting AxonHub Desktop Agent v{}", env!("CARGO_PKG_VERSION"));
    
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let session_id = args.get(1).cloned().unwrap_or_else(|| {
        error!("Usage: axon-desktop-agent <session-id> [hub-url] [grpc-port]");
        std::process::exit(1);
    });
    
    let hub_url = args.get(2)
        .cloned()
        .unwrap_or_else(|| "http://localhost:4545".to_string());
    
    let grpc_port = args.get(3)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(50051);
    
    info!("Connecting to hub at {} for session {}", hub_url, session_id);
    info!("Platform: {}", platform::get_platform_name());
    info!("System info: {:?}", platform::get_system_info()?);
    
    // Start gRPC server (bind to 0.0.0.0 for remote access)
    let addr = format!("0.0.0.0:{}", grpc_port).parse()?;
    let service = DesktopAgentService::server();
    
    info!("Starting gRPC server on {} (accessible remotely)", addr);
    
    // Run gRPC server and agent concurrently
    tokio::select! {
        result = tonic::transport::Server::builder()
            .add_service(service)
            .serve(addr) => {
            if let Err(e) = result {
                error!("gRPC server error: {}", e);
                return Err(e.into());
            }
        }
        
        // Also create agent instance (currently just logs and keeps alive)
        result = async {
            let agent = Agent::new(session_id, hub_url)?;
            agent.run().await
        } => {
            if let Err(e) = result {
                error!("Agent error: {}", e);
                return Err(e);
            }
        }
    }
    
    info!("Agent shutdown complete");
    Ok(())
}

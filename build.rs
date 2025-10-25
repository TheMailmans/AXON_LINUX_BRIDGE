// Build script for proto compilation
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(false) // Bridge is server only
        .compile(
            &["proto/agent.proto"],
            &["proto"],
        )?;
    Ok(())
}

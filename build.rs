/*!
 * Build script for desktop agent
 * 
 * Handles platform-specific linking and code generation.
 */

fn main() {
    // Compile protobuf definitions
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile(&["proto/agent.proto"], &["proto"])
        .expect("Failed to compile protobuf definitions");
    
    // Platform-specific linking based on TARGET, not host
    let target = std::env::var("TARGET").unwrap_or_default();
    
    if target.contains("apple") || target.contains("darwin") {
        // Link CoreAudio frameworks for audio capture (macOS)
        println!("cargo:rustc-link-lib=framework=CoreAudio");
        println!("cargo:rustc-link-lib=framework=AudioToolbox");
        println!("cargo:rustc-link-lib=framework=AudioUnit");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    } else if target.contains("windows") {
        // Link Windows audio libraries
        // WASAPI is part of Windows SDK, no additional linking needed
        println!("cargo:rustc-link-lib=ole32");
        println!("cargo:rustc-link-lib=user32");
    } else if target.contains("linux") {
        // Link PulseAudio libraries (Linux)
        println!("cargo:rustc-link-lib=pulse");
        println!("cargo:rustc-link-lib=pulse-simple");
    }
}

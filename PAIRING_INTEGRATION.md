# Bridge Pairing Integration Guide

## Overview

This document describes the Bridge-side gRPC handlers that need to be implemented to support the dual-GUI pairing system. These handlers will be added to `src/main.rs` in the `DesktopAgent` trait implementation.

## Prerequisites

The pairing backend has already been implemented in AxonHub V3 Core:
- ✅ Pairing code generation (ABC-123 format)
- ✅ Code validation with rate limiting
- ✅ JWT token generation/verification
- ✅ Keychain integration
- ✅ 54 passing tests

## Required Dependencies

Add to `Cargo.toml`:

```toml
[dependencies]
# Existing dependencies...
axonhub-v3 = { path = "../AxonHubV3", features = ["pairing"] }
chrono = "0.4"
uuid = "1.0"
```

## Implementation Steps

### Step 1: Update BridgeService State

Add these fields to the `BridgeService` struct in `src/main.rs`:

```rust
use axonhub_v3::pairing::{PairingCodeGenerator, PairingCodeStore, PairingValidator};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct BridgeService {
    // ... existing fields ...

    /// Pairing code generator
    code_generator: Arc<PairingCodeGenerator>,

    /// Pairing code storage (in-memory + database)
    code_storage: Arc<Mutex<PairingCodeStore>>,

    /// Pairing validator with rate limiting
    pairing_validator: Arc<PairingValidator>,

    /// Bridge ID (UUID)
    bridge_id: String,

    /// JWT secret for token signing
    jwt_secret: String,
}
```

### Step 2: Update BridgeService::new()

Initialize pairing components:

```rust
impl BridgeService {
    pub fn new() -> Result<Self> {
        // ... existing initialization ...

        // Generate or load Bridge ID
        let bridge_id = uuid::Uuid::new_v4().to_string();

        // Load JWT secret from environment or generate
        let jwt_secret = std::env::var("AXONHUB_JWT_SECRET")
            .unwrap_or_else(|_| {
                warn!("AXONHUB_JWT_SECRET not set, generating random secret");
                uuid::Uuid::new_v4().to_string()
            });

        // Create pairing components
        let code_generator = Arc::new(PairingCodeGenerator::new());
        let code_storage = Arc::new(Mutex::new(PairingCodeStore::new()));

        let bridge_info = axonhub_v3::pairing::BridgeInfo {
            bridge_id: bridge_id.clone(),
            bridge_name: "Ubuntu Bridge".to_string(), // TODO: Get from config
            platform: "linux".to_string(),
            screen_width: Some(1920), // TODO: Get from system
            screen_height: Some(1080),
        };

        let pairing_validator = Arc::new(PairingValidator::new(
            Arc::clone(&code_storage),
            jwt_secret.clone(),
            bridge_info,
        ));

        Ok(Self {
            // ... existing fields ...
            code_generator,
            code_storage,
            pairing_validator,
            bridge_id,
            jwt_secret,
        })
    }
}
```

### Step 3: Implement Pairing RPC Handlers

Add these three methods to the `#[tonic::async_trait] impl DesktopAgent for BridgeService` block:

```rust
/// Generate a pairing code
async fn generate_pairing_code(
    &self,
    request: Request<PairingCodeRequest>,
) -> Result<Response<PairingCodeResponse>, Status> {
    let req = request.into_inner();

    info!("[Bridge] Generating pairing code for bridge: {}", req.bridge_id);

    // Generate code
    let code = self.code_generator.generate();

    // Store code with 15-minute expiry
    let expires_at = chrono::Utc::now() + chrono::Duration::minutes(15);

    match self.code_storage.lock().await.store_code(code.clone(), 900) {
        Ok(_) => {
            info!("[Bridge] ✅ Pairing code generated: {} (expires: {})", code, expires_at);

            // Show notification on Bridge GUI
            if let Err(e) = notifications::notify_pairing_code_generated(&code) {
                warn!("[Bridge] Failed to show pairing notification: {}", e);
            }

            Ok(Response::new(PairingCodeResponse {
                success: true,
                code: Some(code),
                error: None,
                expires_at: expires_at.timestamp(),
            }))
        }
        Err(e) => {
            error!("[Bridge] ❌ Failed to store pairing code: {}", e);
            Ok(Response::new(PairingCodeResponse {
                success: false,
                code: None,
                error: Some(format!("Failed to store code: {}", e)),
                expires_at: 0,
            }))
        }
    }
}

/// Validate a pairing code
async fn validate_pairing_code(
    &self,
    request: Request<ValidatePairingRequest>,
) -> Result<Response<ValidatePairingResponse>, Status> {
    let req = request.into_inner();
    let client_ip = request
        .remote_addr()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    info!(
        "[Bridge] Validating pairing code {} from Core {} ({})",
        req.code, req.core_name, req.core_id
    );

    // Create CoreInfo
    let core_info = axonhub_v3::pairing::CoreInfo {
        core_id: req.core_id.clone(),
        core_name: req.core_name.clone(),
        core_ip: client_ip,
        core_platform: "unknown".to_string(), // TODO: Get from request
    };

    // Validate code
    match self.pairing_validator.validate_code(&req.code, &core_info) {
        Ok(pairing_token) => {
            info!(
                "[Bridge] ✅ Pairing successful: Core {} ({}) paired with Bridge {}",
                req.core_name, req.core_id, self.bridge_id
            );

            // Show success notification
            if let Err(e) = notifications::notify_pairing_successful(&req.core_name) {
                warn!("[Bridge] Failed to show pairing success notification: {}", e);
            }

            // Update tray: show paired Core
            if let Some(ref tray) = self.tray_handle {
                tray.add_paired_core(&req.core_name).await;
            }

            Ok(Response::new(ValidatePairingResponse {
                success: true,
                token: Some(pairing_token.token),
                bridge_id: Some(pairing_token.bridge_id),
                bridge_name: Some(pairing_token.bridge_name),
                error: None,
            }))
        }
        Err(e) => {
            warn!(
                "[Bridge] ❌ Pairing validation failed for code {}: {}",
                req.code, e
            );

            Ok(Response::new(ValidatePairingResponse {
                success: false,
                token: None,
                bridge_id: None,
                bridge_name: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

/// Revoke a pairing token
async fn revoke_pairing_token(
    &self,
    request: Request<RevokeTokenRequest>,
) -> Result<Response<RevokeTokenResponse>, Status> {
    let req = request.into_inner();

    info!("[Bridge] Revoking pairing token: {}", req.reason);

    // Verify the token first to get the JTI
    let token_verifier = axonhub_v3::pairing::TokenVerifier::new(self.jwt_secret.clone());

    match token_verifier.verify_token(&req.token) {
        Ok(claims) => {
            // Revoke the token
            token_verifier.revoke_token(&claims.jti);

            info!("[Bridge] ✅ Token revoked: {} (Core: {})", claims.jti, claims.core_name);

            // Show notification
            if let Err(e) = notifications::notify_pairing_revoked(&claims.core_name) {
                warn!("[Bridge] Failed to show revocation notification: {}", e);
            }

            // Update tray: remove paired Core
            if let Some(ref tray) = self.tray_handle {
                tray.remove_paired_core(&claims.core_name).await;
            }

            Ok(Response::new(RevokeTokenResponse {
                success: true,
                error: None,
            }))
        }
        Err(e) => {
            warn!("[Bridge] ❌ Failed to revoke token: {}", e);
            Ok(Response::new(RevokeTokenResponse {
                success: false,
                error: Some(e.to_string()),
            }))
        }
    }
}
```

### Step 4: Add Notification Functions

Add these functions to `src/notifications.rs`:

```rust
/// Show pairing code generated notification
pub fn notify_pairing_code_generated(code: &str) -> Result<()> {
    Notification::new()
        .summary("Pairing Code Ready")
        .body(&format!("Enter this code in AxonHub Core:\n\n{}", code))
        .icon("dialog-information")
        .timeout(15000) // 15 seconds
        .show()?;
    Ok(())
}

/// Show pairing successful notification
pub fn notify_pairing_successful(core_name: &str) -> Result<()> {
    Notification::new()
        .summary("Core Paired Successfully")
        .body(&format!("✅ {} is now connected", core_name))
        .icon("dialog-information")
        .timeout(5000)
        .show()?;
    Ok(())
}

/// Show pairing revoked notification
pub fn notify_pairing_revoked(core_name: &str) -> Result<()> {
    Notification::new()
        .summary("Core Unpaired")
        .body(&format!("Connection to {} has been revoked", core_name))
        .icon("dialog-warning")
        .timeout(5000)
        .show()?;
    Ok(())
}
```

### Step 5: Update System Tray (Optional)

Add these methods to `src/system_tray.rs` to show paired Cores in the tray menu:

```rust
impl AxonBridgeTray {
    /// Add a paired Core to the tray menu
    pub async fn add_paired_core(&self, core_name: &str) {
        // TODO: Add menu item showing paired Core
        info!("[Tray] Added paired Core: {}", core_name);
    }

    /// Remove a paired Core from the tray menu
    pub async fn remove_paired_core(&self, core_name: &str) {
        // TODO: Remove menu item for unpaired Core
        info!("[Tray] Removed paired Core: {}", core_name);
    }
}
```

## Environment Variables

The Bridge requires this environment variable:

```bash
export AXONHUB_JWT_SECRET="your-secret-key-here"
```

**IMPORTANT**: Use a strong, randomly generated secret in production. This secret must be kept secure as it's used to sign JWT tokens.

Example generation:
```bash
export AXONHUB_JWT_SECRET=$(openssl rand -hex 32)
```

## Testing the Integration

### 1. Start the Bridge (on Ubuntu VM)

```bash
cd /path/to/AXONBRIDGE-Linux
export AXONHUB_JWT_SECRET="test-secret-123"
cargo run --release
```

### 2. Test from Core (on Mac)

```bash
cd /path/to/AxonHubV3
cargo test --test pairing_integration -- --nocapture
```

### 3. Manual Test Flow

1. **Bridge**: Generate code via gRPC:
   ```bash
   grpcurl -plaintext -d '{"bridge_id":"bridge-123"}' \
     192.168.64.5:50051 \
     axon.agent.DesktopAgent/GeneratePairingCode
   ```

2. **Core**: Validate code:
   ```bash
   grpcurl -plaintext -d '{"code":"ABC-234","core_id":"core-456","core_name":"Test Core"}' \
     192.168.64.5:50051 \
     axon.agent.DesktopAgent/ValidatePairingCode
   ```

3. **Core**: Store token in keychain
4. **Core**: Use token for subsequent gRPC calls
5. **Core**: Revoke token when done

## Security Considerations

1. **JWT Secret**: Never commit the JWT secret to version control. Use environment variables.
2. **HTTPS**: In production, use TLS/SSL for gRPC connections.
3. **Rate Limiting**: The `PairingValidator` includes built-in rate limiting (5 attempts per minute per IP).
4. **Code Expiry**: Pairing codes expire after 15 minutes.
5. **Token Expiry**: JWT tokens expire after 30 days.
6. **Revocation**: Tokens can be revoked immediately via the `RevokeTokenRequest` RPC.

## Database Schema (Future Enhancement)

Currently, pairing codes and tokens are stored in-memory. For production, add these tables:

```sql
CREATE TABLE pairing_codes (
    code VARCHAR(7) PRIMARY KEY,
    bridge_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    used BOOLEAN NOT NULL DEFAULT FALSE,
    used_at TIMESTAMPTZ,
    core_id UUID
);

CREATE TABLE pairing_tokens (
    token_jti UUID PRIMARY KEY,
    bridge_id UUID NOT NULL,
    core_id UUID NOT NULL,
    issued_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE revoked_tokens (
    token_jti UUID PRIMARY KEY,
    revoked_at TIMESTAMPTZ NOT NULL,
    revoked_by VARCHAR(255) NOT NULL,
    reason TEXT
);
```

## Troubleshooting

### Code validation fails with "Code not found"
- Check that the code hasn't expired (15-minute TTL)
- Verify the code format is correct (ABC-123)
- Ensure the Bridge is running and accessible

### Token verification fails
- Check that `AXONHUB_JWT_SECRET` is the same on Bridge and Core
- Verify the token hasn't expired (30-day TTL)
- Check if the token has been revoked

### Rate limit exceeded
- Wait 60 seconds and try again
- Check for multiple failed attempts from the same IP
- Rate limit is 5 attempts per minute per IP

## Next Steps

After implementing these handlers:

1. ✅ Build and test the Bridge on Ubuntu VM
2. ✅ Run integration tests from Core
3. ✅ Implement GUI pairing flow (Phase 10, Week 3)
4. ✅ Add persistent storage for tokens (database)
5. ✅ Implement TLS/SSL for production

## References

- Core pairing backend: `/Users/tylermailman/Documents/Projects/AxonHubV3/src/pairing/`
- Core gRPC client: `/Users/tylermailman/Documents/Projects/AxonHubV3/src/bridge/grpc_client.rs`
- Proto definitions: `/Users/tylermailman/Documents/Projects/AXONBRIDGE-Linux/proto/agent.proto`
- Integration tests: `/Users/tylermailman/Documents/Projects/AxonHubV3/tests/pairing_integration.rs`

/*!
 * Video Encoder Module
 * 
 * Provides H.264 encoding with hardware acceleration when available.
 * Falls back to software encoding on unsupported platforms.
 */

use anyhow::{Result, Context};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn, debug};

use super::frame::{RawFrame, PixelFormat};
use super::{Quality, Codec};

#[cfg(target_os = "macos")]
use crate::video::videotoolbox_ffi::{
    VTCompressionSessionRef, VTCompressionSessionCreate, VTCompressionSessionEncodeFrame,
    VTCompressionSessionCompleteFrames, VTCompressionSessionInvalidate,
    VTCompressionSessionPrepareToEncodeFrames, VTSessionSetProperty,
    CVPixelBuffer, kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange,
    CMTime, CMSampleBufferRef, OSStatus, noErr,
    kCMVideoCodecType_H264, extract_nal_units, is_keyframe,
    get_average_bitrate_key, get_max_keyframe_interval_key, get_expected_framerate_key,
    get_realtime_key, get_profile_level_key, get_h264_baseline_autolevel,
};

#[cfg(target_os = "macos")]
use std::sync::{Arc, Mutex, mpsc};

/// Encoder configuration
#[derive(Debug, Clone)]
pub struct EncoderConfig {
    /// Target width in pixels
    pub width: u32,
    /// Target height in pixels
    pub height: u32,
    /// Target framerate (frames per second)
    pub fps: u32,
    /// Encoding quality
    pub quality: Quality,
    /// Codec to use
    pub codec: Codec,
    /// Enable hardware acceleration if available
    pub hardware_accel: bool,
    /// Keyframe interval (GOP size)
    pub keyframe_interval: u32,
}

impl Default for EncoderConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps: 30,
            quality: Quality::Medium,
            codec: Codec::H264,
            hardware_accel: true,
            keyframe_interval: 60, // Every 2 seconds at 30fps
        }
    }
}

impl EncoderConfig {
    /// Get bitrate for this configuration
    pub fn bitrate_kbps(&self) -> u32 {
        self.quality.bitrate_kbps(self.width, self.height)
    }
}

/// Encoded video frame
#[derive(Debug, Clone)]
pub struct EncodedFrame {
    /// Encoded frame data
    pub data: Vec<u8>,
    /// Timestamp in milliseconds since epoch
    pub timestamp_ms: u64,
    /// Frame sequence number
    pub sequence: u64,
    /// Whether this is a keyframe (I-frame)
    pub is_keyframe: bool,
    /// Presentation timestamp
    pub pts: u64,
    /// Decode timestamp
    pub dts: u64,
    /// Frame width
    pub width: u32,
    /// Frame height
    pub height: u32,
}

/// Video encoder trait
pub trait VideoEncoder: Send {
    /// Encode a raw frame
    fn encode(&mut self, frame: &RawFrame) -> Result<EncodedFrame>;
    
    /// Request a keyframe on the next encode
    fn request_keyframe(&mut self);
    
    /// Get encoder configuration
    fn config(&self) -> &EncoderConfig;
    
    /// Check if hardware acceleration is active
    fn is_hardware_accelerated(&self) -> bool;
}

/// Create a video encoder based on platform and configuration
pub fn create_encoder(config: EncoderConfig) -> Result<Box<dyn VideoEncoder>> {
    info!("Creating video encoder: {:?}", config);
    
    #[cfg(target_os = "macos")]
    {
        if config.hardware_accel {
            info!("Attempting to create VideoToolbox hardware encoder");
            match VideoToolboxEncoder::new(config.clone()) {
                Ok(encoder) => {
                    info!("VideoToolbox hardware encoder created successfully");
                    return Ok(Box::new(encoder));
                }
                Err(e) => {
                    warn!("Failed to create VideoToolbox encoder: {}, falling back to software", e);
                }
            }
        }
    }
    
    // Software fallback
    #[cfg(feature = "h264")]
    {
        info!("Creating software H.264 encoder");
        Ok(Box::new(SoftwareEncoder::new(config)?))
    }
    
    #[cfg(not(feature = "h264"))]
    {
        anyhow::bail!("No encoder available - enable 'h264' feature");
    }
}

// ============================================================================
// macOS VideoToolbox Hardware Encoder
// ============================================================================

/// Callback context for receiving compressed frames
#[cfg(target_os = "macos")]
struct CallbackContext {
    tx: mpsc::Sender<CompressedFrame>,
}

/// Compressed frame from VideoToolbox callback
#[cfg(target_os = "macos")]
struct CompressedFrame {
    data: Vec<u8>,
    timestamp_ms: u64,
    is_keyframe: bool,
}

#[cfg(target_os = "macos")]
pub struct VideoToolboxEncoder {
    config: EncoderConfig,
    session: VTCompressionSessionRef,
    frame_count: u64,
    request_keyframe_flag: bool,
    compressed_rx: Arc<Mutex<mpsc::Receiver<CompressedFrame>>>,
    _callback_context: Arc<CallbackContext>,
}

// VTCompressionSessionRef is an opaque pointer that is thread-safe
#[cfg(target_os = "macos")]
unsafe impl Send for VideoToolboxEncoder {}

#[cfg(target_os = "macos")]
impl VideoToolboxEncoder {
    pub fn new(config: EncoderConfig) -> Result<Self> {
        use std::os::raw::c_void;
        
        info!("Initializing VideoToolbox encoder: {}x{} @ {} fps",
              config.width, config.height, config.fps);
        
        // Create callback channel
        let (compressed_tx, compressed_rx) = mpsc::channel();
        let compressed_rx = Arc::new(Mutex::new(compressed_rx));
        
        // Create callback context
        let callback_context = Arc::new(CallbackContext {
            tx: compressed_tx,
        });
        
        // Get raw pointer to context (will be passed to C callback)
        let callback_context_ptr = Arc::as_ptr(&callback_context) as *mut c_void;
        
        // Create compression session
        let mut session: VTCompressionSessionRef = std::ptr::null_mut();
        let status = unsafe {
            VTCompressionSessionCreate(
                std::ptr::null(), // kCFAllocatorDefault
                config.width as i32,
                config.height as i32,
                kCMVideoCodecType_H264,
                std::ptr::null(), // encoder specification (use default)
                std::ptr::null(), // source image buffer attributes
                std::ptr::null(), // compressed data allocator
                compression_output_callback,
                callback_context_ptr,
                &mut session,
            )
        };
        
        if status != noErr || session.is_null() {
            anyhow::bail!("Failed to create VTCompressionSession: status={}", status);
        }
        
        // Configure session properties
        Self::configure_session(session, &config)
            .context("Failed to configure compression session")?;
        
        // Prepare to encode
        let status = unsafe { VTCompressionSessionPrepareToEncodeFrames(session) };
        if status != noErr {
            unsafe { VTCompressionSessionInvalidate(session) };
            anyhow::bail!("Failed to prepare compression session: status={}", status);
        }
        
        info!("VideoToolbox encoder created successfully");
        
        Ok(Self {
            config,
            session,
            frame_count: 0,
            request_keyframe_flag: false,
            compressed_rx,
            _callback_context: callback_context,
        })
    }
    
    /// Configure VideoToolbox session properties
    fn configure_session(session: VTCompressionSessionRef, config: &EncoderConfig) -> Result<()> {
        use core_foundation::number::CFNumber;
        use core_foundation::boolean::CFBoolean;
        use core_foundation::base::TCFType;
        
        // Set average bitrate
        let bitrate_bps = config.bitrate_kbps() * 1000;
        let bitrate_num = CFNumber::from(bitrate_bps as i32);
        let bitrate_key = get_average_bitrate_key();
        unsafe {
            VTSessionSetProperty(
                session,
                bitrate_key.as_concrete_TypeRef() as *const std::os::raw::c_void,
                bitrate_num.as_CFTypeRef() as *const std::os::raw::c_void,
            );
        }
        
        // Set max keyframe interval
        let keyframe_num = CFNumber::from(config.keyframe_interval as i32);
        let keyframe_key = get_max_keyframe_interval_key();
        unsafe {
            VTSessionSetProperty(
                session,
                keyframe_key.as_concrete_TypeRef() as *const std::os::raw::c_void,
                keyframe_num.as_CFTypeRef() as *const std::os::raw::c_void,
            );
        }
        
        // Set expected framerate
        let fps_num = CFNumber::from(config.fps as i32);
        let fps_key = get_expected_framerate_key();
        unsafe {
            VTSessionSetProperty(
                session,
                fps_key.as_concrete_TypeRef() as *const std::os::raw::c_void,
                fps_num.as_CFTypeRef() as *const std::os::raw::c_void,
            );
        }
        
        // Enable real-time encoding
        let realtime_bool = CFBoolean::true_value();
        let realtime_key = get_realtime_key();
        unsafe {
            VTSessionSetProperty(
                session,
                realtime_key.as_concrete_TypeRef() as *const std::os::raw::c_void,
                realtime_bool.as_CFTypeRef() as *const std::os::raw::c_void,
            );
        }
        
        // Set profile level (baseline for compatibility)
        let profile = get_h264_baseline_autolevel();
        let profile_key = get_profile_level_key();
        unsafe {
            VTSessionSetProperty(
                session,
                profile_key.as_concrete_TypeRef() as *const std::os::raw::c_void,
                profile.as_CFTypeRef() as *const std::os::raw::c_void,
            );
        }
        
        info!("VideoToolbox session configured: bitrate={}kbps, keyframe_interval={}",
              config.bitrate_kbps(), config.keyframe_interval);
        
        Ok(())
    }
    
    /// Convert BGRA raw frame to NV12 pixel buffer
    fn raw_frame_to_pixel_buffer(&self, frame: &RawFrame) -> Result<CVPixelBuffer> {
        let pixel_buffer = CVPixelBuffer::new(
            frame.width,
            frame.height,
            kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange,
        )
        .map_err(|e| anyhow::anyhow!("Failed to create pixel buffer: status={}", e))?;
        
        pixel_buffer.lock(0)
            .map_err(|e| anyhow::anyhow!("Failed to lock pixel buffer: status={}", e))?;
        
        self.convert_bgra_to_nv12(
            &frame.data,
            pixel_buffer.get_y_plane(),
            pixel_buffer.get_uv_plane(),
            frame.width,
            frame.height,
            pixel_buffer.get_y_bytes_per_row(),
            pixel_buffer.get_uv_bytes_per_row(),
        )?;
        
        pixel_buffer.unlock(0)
            .map_err(|e| anyhow::anyhow!("Failed to unlock pixel buffer: status={}", e))?;
        
        Ok(pixel_buffer)
    }
    
    /// Convert BGRA to NV12 (YUV 4:2:0 with interleaved UV)
    /// BT.601 color space conversion for maximum compatibility
    fn convert_bgra_to_nv12(
        &self,
        bgra: &[u8],
        y_plane: *mut u8,
        uv_plane: *mut u8,
        width: u32,
        height: u32,
        y_stride: usize,
        uv_stride: usize,
    ) -> Result<()> {
        let width = width as usize;
        let height = height as usize;
        
        unsafe {
            for y in 0..height {
                for x in 0..width {
                    let bgra_offset = (y * width + x) * 4;
                    if bgra_offset + 3 >= bgra.len() {
                        continue;
                    }
                    
                    let b = bgra[bgra_offset] as f32;
                    let g = bgra[bgra_offset + 1] as f32;
                    let r = bgra[bgra_offset + 2] as f32;
                    
                    // BT.601 conversion: Y = 0.257*R + 0.504*G + 0.098*B + 16
                    let y_val = (0.257 * r + 0.504 * g + 0.098 * b + 16.0)
                        .clamp(0.0, 255.0) as u8;
                    
                    let y_offset = y * y_stride + x;
                    *y_plane.add(y_offset) = y_val;
                    
                    // Subsample UV every 2x2 block
                    if x % 2 == 0 && y % 2 == 0 {
                        let u_val = (-0.148 * r - 0.291 * g + 0.439 * b + 128.0)
                            .clamp(0.0, 255.0) as u8;
                        let v_val = (0.439 * r - 0.368 * g - 0.071 * b + 128.0)
                            .clamp(0.0, 255.0) as u8;
                        
                        let uv_offset = (y / 2) * uv_stride + (x / 2) * 2;
                        *uv_plane.add(uv_offset) = u_val;
                        *uv_plane.add(uv_offset + 1) = v_val;
                    }
                }
            }
        }
        
        Ok(())
    }
}

#[cfg(target_os = "macos")]
impl VideoEncoder for VideoToolboxEncoder {
    fn encode(&mut self, frame: &RawFrame) -> Result<EncodedFrame> {
        use std::time::Duration;
        
        self.frame_count += 1;
        
        debug!("VideoToolbox encoding frame {}", self.frame_count);
        
        let pixel_buffer = self.raw_frame_to_pixel_buffer(frame)
            .context("Failed to convert frame to pixel buffer")?;
        
        let pts = CMTime::new(self.frame_count as i64, self.config.fps as i32);
        let duration = CMTime::new(1, self.config.fps as i32);
        
        let status = unsafe {
            VTCompressionSessionEncodeFrame(
                self.session,
                pixel_buffer.as_raw(),
                pts,
                duration,
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        
        if status != noErr {
            anyhow::bail!("VTCompressionSessionEncodeFrame failed: status={}", status);
        }
        
        let compressed_rx = self.compressed_rx.lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock compressed_rx: {}", e))?;
        
        match compressed_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(compressed) => {
                debug!("Received compressed frame: {} bytes, keyframe: {}",
                       compressed.data.len(), compressed.is_keyframe);
                
                Ok(EncodedFrame {
                    data: compressed.data,
                    timestamp_ms: compressed.timestamp_ms,
                    sequence: self.frame_count,
                    is_keyframe: compressed.is_keyframe,
                    pts: self.frame_count,
                    dts: self.frame_count,
                    width: self.config.width,
                    height: self.config.height,
                })
            }
            Err(e) => {
                anyhow::bail!("Timeout waiting for compressed frame: {}", e)
            }
        }
    }
    
    fn request_keyframe(&mut self) {
        self.request_keyframe_flag = true;
    }
    
    fn config(&self) -> &EncoderConfig {
        &self.config
    }
    
    fn is_hardware_accelerated(&self) -> bool {
        true
    }
}

/// C callback function for VideoToolbox compression output
#[cfg(target_os = "macos")]
extern "C" fn compression_output_callback(
    output_callback_ref_con: *mut std::os::raw::c_void,
    _source_frame_ref_con: *mut std::os::raw::c_void,
    status: OSStatus,
    _info_flags: u32,
    sample_buffer: CMSampleBufferRef,
) {
    use tracing::error;
    
    if status != noErr {
        error!("Compression callback error: status={}", status);
        return;
    }
    
    if sample_buffer.is_null() {
        error!("Compression callback: null sample buffer");
        return;
    }
    
    let context = unsafe { &*(output_callback_ref_con as *const CallbackContext) };
    
    let data = match unsafe { extract_nal_units(sample_buffer) } {
        Ok(d) => d,
        Err(e) => {
            error!("Failed to extract NAL units: status={}", e);
            return;
        }
    };
    
    let is_keyframe = unsafe { is_keyframe(sample_buffer) };
    
    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    
    let compressed = CompressedFrame {
        data,
        timestamp_ms,
        is_keyframe,
    };
    
    if let Err(e) = context.tx.send(compressed) {
        error!("Failed to send compressed frame: {}", e);
    }
}

/// Cleanup VideoToolbox session on drop
#[cfg(target_os = "macos")]
impl Drop for VideoToolboxEncoder {
    fn drop(&mut self) {
        unsafe {
            let _ = VTCompressionSessionCompleteFrames(
                self.session,
                CMTime::invalid(),
            );
            
            VTCompressionSessionInvalidate(self.session);
        }
        
        info!("VideoToolbox encoder destroyed");
    }
}

// ============================================================================
// Software Encoder (OpenH264)
// ============================================================================

#[cfg(feature = "h264")]
pub struct SoftwareEncoder {
    config: EncoderConfig,
    frame_count: u64,
    request_keyframe_flag: bool,
}

#[cfg(feature = "h264")]
impl SoftwareEncoder {
    pub fn new(config: EncoderConfig) -> Result<Self> {
        info!("Initializing software H.264 encoder");
        
        // TODO: Initialize OpenH264 encoder
        // For now, create stub
        
        Ok(Self {
            config,
            frame_count: 0,
            request_keyframe_flag: false,
        })
    }
}

#[cfg(feature = "h264")]
impl VideoEncoder for SoftwareEncoder {
    fn encode(&mut self, frame: &RawFrame) -> Result<EncodedFrame> {
        self.frame_count += 1;
        
        // TODO: Actual software encoding
        // - Convert frame to YUV420 if needed
        // - Encode with OpenH264
        // - Return encoded data
        
        debug!("Software encode frame {} (stub)", self.frame_count);
        
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        Ok(EncodedFrame {
            data: vec![],
            timestamp_ms,
            sequence: self.frame_count,
            is_keyframe: self.frame_count % self.config.keyframe_interval as u64 == 1,
            pts: self.frame_count,
            dts: self.frame_count,
            width: self.config.width,
            height: self.config.height,
        })
    }
    
    fn request_keyframe(&mut self) {
        self.request_keyframe_flag = true;
    }
    
    fn config(&self) -> &EncoderConfig {
        &self.config
    }
    
    fn is_hardware_accelerated(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::video::frame::PixelFormat;

    #[test]
    fn test_encoder_config() {
        let config = EncoderConfig::default();
        assert_eq!(config.width, 1920);
        assert_eq!(config.height, 1080);
        assert_eq!(config.fps, 30);
        assert!(config.hardware_accel);
    }

    #[test]
    fn test_encoder_creation() {
        let config = EncoderConfig::default();
        let encoder = create_encoder(config);
        assert!(encoder.is_ok());
    }

    #[test]
    fn test_quality_bitrate() {
        let config = EncoderConfig {
            quality: Quality::Medium,
            ..Default::default()
        };
        
        let bitrate = config.bitrate_kbps();
        assert!(bitrate > 1000); // Should be reasonable for 1080p medium quality
        assert!(bitrate < 5000); // But not too high
    }
    
    #[test]
    fn test_quality_bitrate_scaling() {
        let low = EncoderConfig {
            quality: Quality::Low,
            ..Default::default()
        };
        let medium = EncoderConfig {
            quality: Quality::Medium,
            ..Default::default()
        };
        let high = EncoderConfig {
            quality: Quality::High,
            ..Default::default()
        };
        
        assert!(low.bitrate_kbps() < medium.bitrate_kbps());
        assert!(medium.bitrate_kbps() < high.bitrate_kbps());
    }
    
    #[test]
    fn test_encoded_frame_has_dimensions() {
        let frame = EncodedFrame {
            data: vec![1, 2, 3, 4],
            timestamp_ms: 1000,
            sequence: 1,
            is_keyframe: true,
            pts: 1,
            dts: 1,
            width: 1920,
            height: 1080,
        };
        
        assert_eq!(frame.width, 1920);
        assert_eq!(frame.height, 1080);
        assert!(frame.is_keyframe);
    }
    
    #[cfg(target_os = "macos")]
    #[test]
    fn test_videotoolbox_encoder_creation() {
        let config = EncoderConfig {
            width: 1280,
            height: 720,
            fps: 30,
            codec: Codec::H264,
            quality: Quality::Medium,
            hardware_accel: true,
            keyframe_interval: 30,
        };
        
        let encoder = VideoToolboxEncoder::new(config);
        assert!(encoder.is_ok(), "VideoToolbox encoder creation should succeed");
        
        let encoder = encoder.unwrap();
        assert_eq!(encoder.config.width, 1280);
        assert_eq!(encoder.config.height, 720);
        assert_eq!(encoder.frame_count, 0);
    }
    
    #[cfg(target_os = "macos")]
    #[test]
    fn test_videotoolbox_encoder_implements_send() {
        fn assert_send<T: Send>() {}
        assert_send::<VideoToolboxEncoder>();
    }
    
    #[cfg(target_os = "macos")]
    #[test]
    fn test_create_test_frame() {
        let width = 640u32;
        let height = 480u32;
        let size = (width * height * 4) as usize;
        
        let mut data = vec![0u8; size];
        
        // Fill with gradient pattern
        for y in 0..height {
            for x in 0..width {
                let offset = ((y * width + x) * 4) as usize;
                data[offset] = (x % 256) as u8;     // B
                data[offset + 1] = (y % 256) as u8; // G
                data[offset + 2] = 128;             // R
                data[offset + 3] = 255;             // A
            }
        }
        
        let frame = RawFrame {
            data,
            width,
            height,
            format: PixelFormat::BGRA,
            timestamp_ms: 0,
            sequence: 0,
        };
        
        assert_eq!(frame.data.len(), size);
        assert_eq!(frame.width, width);
        assert_eq!(frame.height, height);
    }
}

/*!
 * VideoToolbox FFI Bindings
 * 
 * Low-level FFI to VideoToolbox C API for H.264 hardware encoding on macOS.
 * Provides safe Rust wrappers around VTCompressionSession.
 * 
 * VideoToolbox provides hardware-accelerated video encoding/decoding on macOS.
 * This module exposes the necessary APIs for creating an H.264 encoder.
 */

#![cfg(target_os = "macos")]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use core_foundation::base::{CFTypeRef, TCFType, Boolean};
use core_foundation::string::CFString;
use std::os::raw::c_void;
use std::ptr;

// ============================================================================
// Core Media Types
// ============================================================================

/// CMTime - Core Media timestamp
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CMTime {
    pub value: i64,
    pub timescale: i32,
    pub flags: u32,
    pub epoch: i64,
}

impl CMTime {
    /// Create a new CMTime with value and timescale
    pub fn new(value: i64, timescale: i32) -> Self {
        Self {
            value,
            timescale,
            flags: 1, // kCMTimeFlags_Valid
            epoch: 0,
        }
    }
    
    /// Create an invalid CMTime
    pub fn invalid() -> Self {
        Self {
            value: 0,
            timescale: 0,
            flags: 0, // kCMTimeFlags_Invalid
            epoch: 0,
        }
    }
}

/// CMSampleBuffer opaque type
#[repr(C)]
pub struct OpaqueCMSampleBuffer;
pub type CMSampleBufferRef = *mut OpaqueCMSampleBuffer;

/// CMBlockBuffer opaque type  
#[repr(C)]
pub struct OpaqueCMBlockBuffer;
pub type CMBlockBufferRef = *mut OpaqueCMBlockBuffer;

// ============================================================================
// Core Video Types (CVPixelBuffer)
// ============================================================================

/// CVPixelBuffer opaque type
#[repr(C)]
pub struct OpaqueCVPixelBuffer;
pub type CVPixelBufferRef = *mut OpaqueCVPixelBuffer;

/// CVPixelBuffer lock flags
pub type CVPixelBufferLockFlags = u64;
pub const kCVPixelBufferLock_ReadOnly: CVPixelBufferLockFlags = 0x00000001;

/// Pixel format types
pub type OSType = u32;
pub const kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange: OSType = 0x34323076; // '420v' (NV12)
pub const kCVPixelFormatType_32BGRA: OSType = 0x42475241; // 'BGRA'

// ============================================================================
// VideoToolbox Types
// ============================================================================

/// VTCompressionSession opaque type
#[repr(C)]
pub struct OpaqueVTCompressionSession;
pub type VTCompressionSessionRef = *mut OpaqueVTCompressionSession;

/// VideoToolbox status codes
pub type OSStatus = i32;
pub const noErr: OSStatus = 0;

/// Video codec type
pub const kCMVideoCodecType_H264: u32 = 0x61766331; // 'avc1'

/// Compression output callback
pub type VTCompressionOutputCallback = extern "C" fn(
    outputCallbackRefCon: *mut c_void,
    sourceFrameRefCon: *mut c_void,
    status: OSStatus,
    infoFlags: u32,
    sampleBuffer: CMSampleBufferRef,
);

// ============================================================================
// VideoToolbox Property Keys (as raw CFStrings)
// ============================================================================

/// Get average bitrate property key
pub fn get_average_bitrate_key() -> CFString {
    CFString::from_static_string("AverageBitRate")
}

/// Get max keyframe interval property key
pub fn get_max_keyframe_interval_key() -> CFString {
    CFString::from_static_string("MaxKeyFrameInterval")
}

/// Get real-time encoding property key
pub fn get_realtime_key() -> CFString {
    CFString::from_static_string("RealTime")
}

/// Get profile level property key
pub fn get_profile_level_key() -> CFString {
    CFString::from_static_string("ProfileLevel")
}

/// Get expected framerate property key
pub fn get_expected_framerate_key() -> CFString {
    CFString::from_static_string("ExpectedFrameRate")
}

/// Get H.264 baseline auto level value
pub fn get_h264_baseline_autolevel() -> CFString {
    CFString::from_static_string("H264_Baseline_AutoLevel")
}

// ============================================================================
// VideoToolbox C API Bindings
// ============================================================================

#[link(name = "VideoToolbox", kind = "framework")]
extern "C" {
    /// Create a compression session
    pub fn VTCompressionSessionCreate(
        allocator: CFTypeRef,
        width: i32,
        height: i32,
        codecType: u32,
        encoderSpecification: CFTypeRef,
        sourceImageBufferAttributes: CFTypeRef,
        compressedDataAllocator: CFTypeRef,
        outputCallback: VTCompressionOutputCallback,
        outputCallbackRefCon: *mut c_void,
        compressionSessionOut: *mut VTCompressionSessionRef,
    ) -> OSStatus;
    
    /// Encode a frame
    pub fn VTCompressionSessionEncodeFrame(
        session: VTCompressionSessionRef,
        imageBuffer: CVPixelBufferRef,
        presentationTimeStamp: CMTime,
        duration: CMTime,
        frameProperties: CFTypeRef,
        sourceFrameRefCon: *mut c_void,
        infoFlagsOut: *mut u32,
    ) -> OSStatus;
    
    /// Complete all pending frames
    pub fn VTCompressionSessionCompleteFrames(
        session: VTCompressionSessionRef,
        completeUntilPresentationTimeStamp: CMTime,
    ) -> OSStatus;
    
    /// Invalidate the session
    pub fn VTCompressionSessionInvalidate(
        session: VTCompressionSessionRef,
    );
    
    /// Prepare to encode frames
    pub fn VTCompressionSessionPrepareToEncodeFrames(
        session: VTCompressionSessionRef,
    ) -> OSStatus;
    
    /// Set a property on the session
    pub fn VTSessionSetProperty(
        session: VTCompressionSessionRef,
        propertyKey: *const c_void,
        propertyValue: *const c_void,
    ) -> OSStatus;
}

// ============================================================================
// Core Video C API Bindings
// ============================================================================

#[link(name = "CoreVideo", kind = "framework")]
extern "C" {
    /// Create a pixel buffer
    pub fn CVPixelBufferCreate(
        allocator: CFTypeRef,
        width: usize,
        height: usize,
        pixelFormatType: OSType,
        pixelBufferAttributes: CFTypeRef,
        pixelBufferOut: *mut CVPixelBufferRef,
    ) -> OSStatus;
    
    /// Lock pixel buffer base address
    pub fn CVPixelBufferLockBaseAddress(
        pixelBuffer: CVPixelBufferRef,
        lockFlags: CVPixelBufferLockFlags,
    ) -> OSStatus;
    
    /// Unlock pixel buffer base address
    pub fn CVPixelBufferUnlockBaseAddress(
        pixelBuffer: CVPixelBufferRef,
        unlockFlags: CVPixelBufferLockFlags,
    ) -> OSStatus;
    
    /// Get base address of a specific plane
    pub fn CVPixelBufferGetBaseAddressOfPlane(
        pixelBuffer: CVPixelBufferRef,
        planeIndex: usize,
    ) -> *mut c_void;
    
    /// Get bytes per row of a specific plane
    pub fn CVPixelBufferGetBytesPerRowOfPlane(
        pixelBuffer: CVPixelBufferRef,
        planeIndex: usize,
    ) -> usize;
    
    /// Get pixel buffer width
    pub fn CVPixelBufferGetWidth(
        pixelBuffer: CVPixelBufferRef,
    ) -> usize;
    
    /// Get pixel buffer height
    pub fn CVPixelBufferGetHeight(
        pixelBuffer: CVPixelBufferRef,
    ) -> usize;
    
    /// Release a pixel buffer
    pub fn CVPixelBufferRelease(
        pixelBuffer: CVPixelBufferRef,
    );
}

// ============================================================================
// Core Media C API Bindings
// ============================================================================

#[link(name = "CoreMedia", kind = "framework")]
extern "C" {
    /// Get block buffer from sample buffer
    pub fn CMSampleBufferGetDataBuffer(
        sbuf: CMSampleBufferRef,
    ) -> CMBlockBufferRef;
    
    /// Get data pointer from block buffer
    pub fn CMBlockBufferGetDataPointer(
        theBuffer: CMBlockBufferRef,
        offset: usize,
        lengthAtOffsetOut: *mut usize,
        totalLengthOut: *mut usize,
        dataPointerOut: *mut *mut c_void,
    ) -> OSStatus;
    
    /// Get total data length
    pub fn CMBlockBufferGetDataLength(
        theBuffer: CMBlockBufferRef,
    ) -> usize;
    
    /// Check if sample buffer contains a keyframe
    pub fn CMSampleBufferGetSampleAttachmentsArray(
        sbuf: CMSampleBufferRef,
        createIfNecessary: Boolean,
    ) -> CFTypeRef; // Actually CFArrayRef
}

// ============================================================================
// Safe Rust Wrappers
// ============================================================================

/// Safe wrapper for CVPixelBuffer
pub struct CVPixelBuffer {
    inner: CVPixelBufferRef,
}

impl CVPixelBuffer {
    /// Create a new pixel buffer
    pub fn new(width: u32, height: u32, format: OSType) -> Result<Self, OSStatus> {
        unsafe {
            let mut pixel_buffer: CVPixelBufferRef = ptr::null_mut();
            let status = CVPixelBufferCreate(
                ptr::null(), // kCFAllocatorDefault
                width as usize,
                height as usize,
                format,
                ptr::null(), // no attributes
                &mut pixel_buffer,
            );
            
            if status != noErr || pixel_buffer.is_null() {
                return Err(status);
            }
            
            Ok(Self {
                inner: pixel_buffer,
            })
        }
    }
    
    /// Lock the pixel buffer for reading/writing
    pub fn lock(&self, flags: CVPixelBufferLockFlags) -> Result<(), OSStatus> {
        unsafe {
            let status = CVPixelBufferLockBaseAddress(self.inner, flags);
            if status != noErr {
                return Err(status);
            }
            Ok(())
        }
    }
    
    /// Unlock the pixel buffer
    pub fn unlock(&self, flags: CVPixelBufferLockFlags) -> Result<(), OSStatus> {
        unsafe {
            let status = CVPixelBufferUnlockBaseAddress(self.inner, flags);
            if status != noErr {
                return Err(status);
            }
            Ok(())
        }
    }
    
    /// Get base address of Y plane (plane 0) for NV12 format
    pub fn get_y_plane(&self) -> *mut u8 {
        unsafe {
            CVPixelBufferGetBaseAddressOfPlane(self.inner, 0) as *mut u8
        }
    }
    
    /// Get base address of UV plane (plane 1) for NV12 format
    pub fn get_uv_plane(&self) -> *mut u8 {
        unsafe {
            CVPixelBufferGetBaseAddressOfPlane(self.inner, 1) as *mut u8
        }
    }
    
    /// Get bytes per row for Y plane
    pub fn get_y_bytes_per_row(&self) -> usize {
        unsafe {
            CVPixelBufferGetBytesPerRowOfPlane(self.inner, 0)
        }
    }
    
    /// Get bytes per row for UV plane
    pub fn get_uv_bytes_per_row(&self) -> usize {
        unsafe {
            CVPixelBufferGetBytesPerRowOfPlane(self.inner, 1)
        }
    }
    
    /// Get the raw CVPixelBufferRef
    pub fn as_raw(&self) -> CVPixelBufferRef {
        self.inner
    }
    
    /// Get width
    pub fn width(&self) -> usize {
        unsafe { CVPixelBufferGetWidth(self.inner) }
    }
    
    /// Get height
    pub fn height(&self) -> usize {
        unsafe { CVPixelBufferGetHeight(self.inner) }
    }
}

impl Drop for CVPixelBuffer {
    fn drop(&mut self) {
        unsafe {
            if !self.inner.is_null() {
                CVPixelBufferRelease(self.inner);
            }
        }
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Extract H.264 NAL units from CMSampleBuffer
pub unsafe fn extract_nal_units(sample_buffer: CMSampleBufferRef) -> Result<Vec<u8>, OSStatus> {
    let block_buffer = CMSampleBufferGetDataBuffer(sample_buffer);
    if block_buffer.is_null() {
        return Err(-1);
    }
    
    let total_length = CMBlockBufferGetDataLength(block_buffer);
    let mut data = vec![0u8; total_length];
    
    let mut data_pointer: *mut c_void = ptr::null_mut();
    let mut length_at_offset: usize = 0;
    let mut total_length_out: usize = 0;
    
    let status = CMBlockBufferGetDataPointer(
        block_buffer,
        0,
        &mut length_at_offset,
        &mut total_length_out,
        &mut data_pointer,
    );
    
    if status != noErr || data_pointer.is_null() {
        return Err(status);
    }
    
    // Copy data
    std::ptr::copy_nonoverlapping(
        data_pointer as *const u8,
        data.as_mut_ptr(),
        total_length,
    );
    
    Ok(data)
}

/// Check if a sample buffer contains a keyframe
pub unsafe fn is_keyframe(sample_buffer: CMSampleBufferRef) -> bool {
    let attachments = CMSampleBufferGetSampleAttachmentsArray(sample_buffer, 0);
    if attachments.is_null() {
        return false;
    }
    
    // For now, conservatively return false
    // Full implementation would parse CFArray and check for kCMSampleAttachmentKey_NotSync
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmtime_creation() {
        let time = CMTime::new(100, 30);
        assert_eq!(time.value, 100);
        assert_eq!(time.timescale, 30);
        assert_eq!(time.flags, 1);
    }

    #[test]
    fn test_cmtime_invalid() {
        let time = CMTime::invalid();
        assert_eq!(time.flags, 0);
    }

    #[test]
    fn test_pixel_buffer_creation() {
        let result = CVPixelBuffer::new(
            1920,
            1080,
            kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange,
        );
        assert!(result.is_ok());
        
        let buffer = result.unwrap();
        assert_eq!(buffer.width(), 1920);
        assert_eq!(buffer.height(), 1080);
    }
}

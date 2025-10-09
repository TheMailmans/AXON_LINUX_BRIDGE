/*!
 * PulseAudio FFI Bindings
 * 
 * Low-level bindings for PulseAudio Simple API.
 * For Linux audio capture.
 */

#![allow(non_camel_case_types)]
#![allow(dead_code)]

use std::ffi::c_void;

// PulseAudio types
pub type pa_sample_format_t = i32;
pub type pa_usec_t = u64;

// Sample formats
pub const PA_SAMPLE_U8: pa_sample_format_t = 0;
pub const PA_SAMPLE_ALAW: pa_sample_format_t = 1;
pub const PA_SAMPLE_ULAW: pa_sample_format_t = 2;
pub const PA_SAMPLE_S16LE: pa_sample_format_t = 3;
pub const PA_SAMPLE_S16BE: pa_sample_format_t = 4;
pub const PA_SAMPLE_FLOAT32LE: pa_sample_format_t = 5;
pub const PA_SAMPLE_FLOAT32BE: pa_sample_format_t = 6;
pub const PA_SAMPLE_S32LE: pa_sample_format_t = 7;
pub const PA_SAMPLE_S32BE: pa_sample_format_t = 8;
pub const PA_SAMPLE_S24LE: pa_sample_format_t = 9;
pub const PA_SAMPLE_S24BE: pa_sample_format_t = 10;
pub const PA_SAMPLE_S24_32LE: pa_sample_format_t = 11;
pub const PA_SAMPLE_S24_32BE: pa_sample_format_t = 12;

// Stream direction
pub const PA_STREAM_PLAYBACK: i32 = 1;
pub const PA_STREAM_RECORD: i32 = 2;

// Sample spec
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct pa_sample_spec {
    pub format: pa_sample_format_t,
    pub rate: u32,
    pub channels: u8,
}

// Buffer attributes
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct pa_buffer_attr {
    pub maxlength: u32,
    pub tlength: u32,
    pub prebuf: u32,
    pub minreq: u32,
    pub fragsize: u32,
}

// Channel map position
pub type pa_channel_position_t = i32;

pub const PA_CHANNEL_POSITION_MONO: pa_channel_position_t = 0;
pub const PA_CHANNEL_POSITION_FRONT_LEFT: pa_channel_position_t = 1;
pub const PA_CHANNEL_POSITION_FRONT_RIGHT: pa_channel_position_t = 2;
pub const PA_CHANNEL_POSITION_FRONT_CENTER: pa_channel_position_t = 3;

// Channel map
#[repr(C)]
#[derive(Debug, Clone)]
pub struct pa_channel_map {
    pub channels: u8,
    pub map: [pa_channel_position_t; 32],
}

// Simple API opaque structure
pub enum pa_simple {}

// Error code
pub type pa_error_code_t = i32;

// PulseAudio Simple API functions
extern "C" {
    /// Create a new connection to the server
    pub fn pa_simple_new(
        server: *const i8,
        name: *const i8,
        dir: i32,
        dev: *const i8,
        stream_name: *const i8,
        ss: *const pa_sample_spec,
        map: *const pa_channel_map,
        attr: *const pa_buffer_attr,
        error: *mut i32,
    ) -> *mut pa_simple;
    
    /// Close and free the connection
    pub fn pa_simple_free(s: *mut pa_simple);
    
    /// Write some data to the server (playback)
    pub fn pa_simple_write(
        s: *mut pa_simple,
        data: *const c_void,
        bytes: usize,
        error: *mut i32,
    ) -> i32;
    
    /// Read some data from the server (record)
    pub fn pa_simple_read(
        s: *mut pa_simple,
        data: *mut c_void,
        bytes: usize,
        error: *mut i32,
    ) -> i32;
    
    /// Wait until all data already written is played
    pub fn pa_simple_drain(
        s: *mut pa_simple,
        error: *mut i32,
    ) -> i32;
    
    /// Return the playback latency
    pub fn pa_simple_get_latency(
        s: *mut pa_simple,
        error: *mut i32,
    ) -> pa_usec_t;
    
    /// Flush the playback buffer
    pub fn pa_simple_flush(
        s: *mut pa_simple,
        error: *mut i32,
    ) -> i32;
    
    /// Return a human readable error message
    pub fn pa_strerror(error: i32) -> *const i8;
}

// Helper function to create stereo channel map
pub fn create_stereo_channel_map() -> pa_channel_map {
    let mut map = pa_channel_map {
        channels: 2,
        map: [0; 32],
    };
    map.map[0] = PA_CHANNEL_POSITION_FRONT_LEFT;
    map.map[1] = PA_CHANNEL_POSITION_FRONT_RIGHT;
    map
}

// Helper function to create float32 sample spec
pub fn create_float32_sample_spec(rate: u32, channels: u8) -> pa_sample_spec {
    pa_sample_spec {
        format: if cfg!(target_endian = "little") {
            PA_SAMPLE_FLOAT32LE
        } else {
            PA_SAMPLE_FLOAT32BE
        },
        rate,
        channels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_spec_creation() {
        let spec = create_float32_sample_spec(48000, 2);
        assert_eq!(spec.rate, 48000);
        assert_eq!(spec.channels, 2);
        assert_eq!(spec.format, PA_SAMPLE_FLOAT32LE);
    }

    #[test]
    fn test_channel_map_creation() {
        let map = create_stereo_channel_map();
        assert_eq!(map.channels, 2);
        assert_eq!(map.map[0], PA_CHANNEL_POSITION_FRONT_LEFT);
        assert_eq!(map.map[1], PA_CHANNEL_POSITION_FRONT_RIGHT);
    }
}

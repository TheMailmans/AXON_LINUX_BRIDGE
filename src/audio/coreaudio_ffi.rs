/*!
 * CoreAudio FFI Bindings for macOS
 * 
 * Provides low-level bindings to macOS CoreAudio framework.
 * Used for system audio loopback and microphone capture.
 */

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use std::ffi::c_void;

// CoreAudio types
pub type OSStatus = i32;
pub type AudioObjectID = u32;
pub type AudioDeviceID = AudioObjectID;
pub type UInt32 = u32;
pub type Float32 = f32;
pub type AudioObjectPropertySelector = u32;
pub type AudioObjectPropertyScope = u32;
pub type AudioObjectPropertyElement = u32;

// Audio Component types
pub type AudioComponent = *mut c_void;
pub type AudioComponentInstance = *mut c_void;
pub type AudioUnit = AudioComponentInstance;

// Constants
pub const kAudioHardwareNoError: OSStatus = 0;

// Audio Object Property Selectors
pub const kAudioHardwarePropertyDefaultOutputDevice: AudioObjectPropertySelector = 0x646f7574; // 'dOut'
pub const kAudioHardwarePropertyDefaultInputDevice: AudioObjectPropertySelector = 0x64496e20; // 'dIn '

// Audio Object Property Scopes
pub const kAudioObjectPropertyScopeGlobal: AudioObjectPropertyScope = 0x676c6f62; // 'glob'
pub const kAudioObjectPropertyScopeInput: AudioObjectPropertyScope = 0x696e7074; // 'inpt'
pub const kAudioObjectPropertyScopeOutput: AudioObjectPropertyScope = 0x6f757470; // 'outp'

// Audio Object Property Elements
pub const kAudioObjectPropertyElementMain: AudioObjectPropertyElement = 0;

// Audio Unit Types
pub const kAudioUnitType_Output: u32 = 0x61756f75; // 'auou'
pub const kAudioUnitType_Mixer: u32 = 0x61756d78; // 'aumx'
pub const kAudioUnitType_Effect: u32 = 0x61756678; // 'aufx'

// Audio Unit Subtypes
pub const kAudioUnitSubType_HALOutput: u32 = 0x6168616c; // 'ahal'
pub const kAudioUnitSubType_DefaultOutput: u32 = 0x64656620; // 'def '
pub const kAudioUnitSubType_SystemOutput: u32 = 0x73797320; // 'sys '

// Audio Unit Properties
pub const kAudioOutputUnitProperty_CurrentDevice: u32 = 2000;
pub const kAudioOutputUnitProperty_EnableIO: u32 = 2003;

// Audio Unit Scopes
pub const kAudioUnitScope_Input: u32 = 1;
pub const kAudioUnitScope_Output: u32 = 2;
pub const kAudioUnitScope_Global: u32 = 0;

// Audio Unit Elements
pub const kAudioUnitElement_Input: u32 = 1;
pub const kAudioUnitElement_Output: u32 = 0;

/// Audio Object Property Address
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AudioObjectPropertyAddress {
    pub mSelector: AudioObjectPropertySelector,
    pub mScope: AudioObjectPropertyScope,
    pub mElement: AudioObjectPropertyElement,
}

/// Audio Stream Basic Description
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AudioStreamBasicDescription {
    pub mSampleRate: f64,
    pub mFormatID: u32,
    pub mFormatFlags: u32,
    pub mBytesPerPacket: u32,
    pub mFramesPerPacket: u32,
    pub mBytesPerFrame: u32,
    pub mChannelsPerFrame: u32,
    pub mBitsPerChannel: u32,
    pub mReserved: u32,
}

// Audio Format Constants
pub const kAudioFormatLinearPCM: u32 = 0x6c70636d; // 'lpcm'
pub const kAudioFormatFlagIsFloat: u32 = 1 << 0;
pub const kAudioFormatFlagIsPacked: u32 = 1 << 3;
pub const kAudioFormatFlagIsNonInterleaved: u32 = 1 << 5;

/// Audio Buffer
#[repr(C)]
#[derive(Debug)]
pub struct AudioBuffer {
    pub mNumberChannels: u32,
    pub mDataByteSize: u32,
    pub mData: *mut c_void,
}

/// Audio Buffer List
#[repr(C)]
#[derive(Debug)]
pub struct AudioBufferList {
    pub mNumberBuffers: u32,
    pub mBuffers: [AudioBuffer; 1], // Variable length array
}

/// Audio Time Stamp
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AudioTimeStamp {
    pub mSampleTime: f64,
    pub mHostTime: u64,
    pub mRateScalar: f64,
    pub mWordClockTime: u64,
    pub mSMPTETime: SMPTETime,
    pub mFlags: u32,
    pub mReserved: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SMPTETime {
    pub mSubframes: i16,
    pub mSubframeDivisor: i16,
    pub mCounter: u32,
    pub mType: u32,
    pub mFlags: u32,
    pub mHours: i16,
    pub mMinutes: i16,
    pub mSeconds: i16,
    pub mFrames: i16,
}

// Audio Unit Render Action Flags
pub type AudioUnitRenderActionFlags = u32;
pub const kAudioUnitRenderAction_PreRender: AudioUnitRenderActionFlags = 1 << 2;
pub const kAudioUnitRenderAction_PostRender: AudioUnitRenderActionFlags = 1 << 3;
pub const kAudioUnitRenderAction_OutputIsSilence: AudioUnitRenderActionFlags = 1 << 4;

/// Audio Unit Render Callback
pub type AURenderCallback = extern "C" fn(
    inRefCon: *mut c_void,
    ioActionFlags: *mut AudioUnitRenderActionFlags,
    inTimeStamp: *const AudioTimeStamp,
    inBusNumber: u32,
    inNumberFrames: u32,
    ioData: *mut AudioBufferList,
) -> OSStatus;

/// Audio Unit Render Callback Struct
#[repr(C)]
#[derive(Debug)]
pub struct AURenderCallbackStruct {
    pub inputProc: AURenderCallback,
    pub inputProcRefCon: *mut c_void,
}

// CoreAudio framework linkage (linked via build.rs)
#[link(name = "CoreAudio", kind = "framework")]
#[link(name = "AudioToolbox", kind = "framework")]
extern "C" {
    // Audio Object API
    pub fn AudioObjectGetPropertyData(
        inObjectID: AudioObjectID,
        inAddress: *const AudioObjectPropertyAddress,
        inQualifierDataSize: u32,
        inQualifierData: *const c_void,
        ioDataSize: *mut u32,
        outData: *mut c_void,
    ) -> OSStatus;
    
    pub fn AudioObjectHasProperty(
        inObjectID: AudioObjectID,
        inAddress: *const AudioObjectPropertyAddress,
    ) -> bool;
    
    // Audio Component API
    pub fn AudioComponentFindNext(
        inComponent: AudioComponent,
        inDesc: *const AudioComponentDescription,
    ) -> AudioComponent;
    
    pub fn AudioComponentInstanceNew(
        inComponent: AudioComponent,
        outInstance: *mut AudioComponentInstance,
    ) -> OSStatus;
    
    pub fn AudioComponentInstanceDispose(
        inInstance: AudioComponentInstance,
    ) -> OSStatus;
    
    // Audio Unit API
    pub fn AudioUnitInitialize(
        inUnit: AudioUnit,
    ) -> OSStatus;
    
    pub fn AudioUnitUninitialize(
        inUnit: AudioUnit,
    ) -> OSStatus;
    
    pub fn AudioUnitSetProperty(
        inUnit: AudioUnit,
        inID: u32,
        inScope: u32,
        inElement: u32,
        inData: *const c_void,
        inDataSize: u32,
    ) -> OSStatus;
    
    pub fn AudioUnitGetProperty(
        inUnit: AudioUnit,
        inID: u32,
        inScope: u32,
        inElement: u32,
        outData: *mut c_void,
        ioDataSize: *mut u32,
    ) -> OSStatus;
    
    pub fn AudioOutputUnitStart(
        inUnit: AudioUnit,
    ) -> OSStatus;
    
    pub fn AudioOutputUnitStop(
        inUnit: AudioUnit,
    ) -> OSStatus;
    
    pub fn AudioUnitRender(
        inUnit: AudioUnit,
        ioActionFlags: *mut AudioUnitRenderActionFlags,
        inTimeStamp: *const AudioTimeStamp,
        inOutputBusNumber: u32,
        inNumberFrames: u32,
        ioData: *mut AudioBufferList,
    ) -> OSStatus;
}

/// Audio Component Description
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AudioComponentDescription {
    pub componentType: u32,
    pub componentSubType: u32,
    pub componentManufacturer: u32,
    pub componentFlags: u32,
    pub componentFlagsMask: u32,
}

// System Object IDs
pub const kAudioObjectSystemObject: AudioObjectID = 1;

// Audio Unit Property IDs
pub const kAudioUnitProperty_StreamFormat: u32 = 8;
pub const kAudioUnitProperty_SetRenderCallback: u32 = 23;
pub const kAudioUnitProperty_EnableIO: u32 = 2003;
pub const kAudioUnitProperty_CurrentDevice: u32 = 2000;

/// Helper function to create AudioStreamBasicDescription for Float32 PCM
pub fn create_float32_pcm_format(
    sample_rate: f64,
    channels: u32,
) -> AudioStreamBasicDescription {
    AudioStreamBasicDescription {
        mSampleRate: sample_rate,
        mFormatID: kAudioFormatLinearPCM,
        mFormatFlags: kAudioFormatFlagIsFloat | kAudioFormatFlagIsPacked,
        mBytesPerPacket: 4 * channels,
        mFramesPerPacket: 1,
        mBytesPerFrame: 4 * channels,
        mChannelsPerFrame: channels,
        mBitsPerChannel: 32,
        mReserved: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_pcm_format() {
        let format = create_float32_pcm_format(48000.0, 2);
        assert_eq!(format.mSampleRate, 48000.0);
        assert_eq!(format.mChannelsPerFrame, 2);
        assert_eq!(format.mFormatID, kAudioFormatLinearPCM);
        assert_eq!(format.mBitsPerChannel, 32);
    }

    #[test]
    fn test_audio_property_address() {
        let addr = AudioObjectPropertyAddress {
            mSelector: kAudioHardwarePropertyDefaultOutputDevice,
            mScope: kAudioObjectPropertyScopeGlobal,
            mElement: kAudioObjectPropertyElementMain,
        };
        
        assert_eq!(addr.mSelector, 0x646f7574);
        assert_eq!(addr.mScope, 0x676c6f62);
        assert_eq!(addr.mElement, 0);
    }
}

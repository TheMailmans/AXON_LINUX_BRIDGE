/*!
 * Windows WASAPI FFI Bindings
 * 
 * Low-level bindings for Windows Audio Session API (WASAPI).
 */

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::ffi::c_void;

// Windows types
pub type HRESULT = i32;
pub type UINT32 = u32;
pub type DWORD = u32;
pub type WORD = u16;
pub type REFERENCE_TIME = i64;
pub type LPWSTR = *mut u16;
pub type LPCWSTR = *const u16;

// GUIDs
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GUID {
    pub Data1: u32,
    pub Data2: u16,
    pub Data3: u16,
    pub Data4: [u8; 8],
}

// COM IUnknown
#[repr(C)]
pub struct IUnknown {
    pub lpVtbl: *const IUnknownVtbl,
}

#[repr(C)]
pub struct IUnknownVtbl {
    pub QueryInterface: unsafe extern "system" fn(
        This: *mut IUnknown,
        riid: *const GUID,
        ppvObject: *mut *mut c_void,
    ) -> HRESULT,
    pub AddRef: unsafe extern "system" fn(This: *mut IUnknown) -> u32,
    pub Release: unsafe extern "system" fn(This: *mut IUnknown) -> u32,
}

// WAVEFORMATEX
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WAVEFORMATEX {
    pub wFormatTag: WORD,
    pub nChannels: WORD,
    pub nSamplesPerSec: DWORD,
    pub nAvgBytesPerSec: DWORD,
    pub nBlockAlign: WORD,
    pub wBitsPerSample: WORD,
    pub cbSize: WORD,
}

// WAVEFORMATEXTENSIBLE
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WAVEFORMATEXTENSIBLE {
    pub Format: WAVEFORMATEX,
    pub Samples: WORD,
    pub dwChannelMask: DWORD,
    pub SubFormat: GUID,
}

// Audio Client Share Mode
pub const AUDCLNT_SHAREMODE_SHARED: i32 = 0;
pub const AUDCLNT_SHAREMODE_EXCLUSIVE: i32 = 1;

// Audio Client Stream Flags
pub const AUDCLNT_STREAMFLAGS_LOOPBACK: u32 = 0x00020000;
pub const AUDCLNT_STREAMFLAGS_EVENTCALLBACK: u32 = 0x00040000;

// Wave format tags
pub const WAVE_FORMAT_PCM: u16 = 0x0001;
pub const WAVE_FORMAT_IEEE_FLOAT: u16 = 0x0003;
pub const WAVE_FORMAT_EXTENSIBLE: u16 = 0xFFFE;

// HRESULT constants
pub const S_OK: HRESULT = 0;
pub const E_FAIL: HRESULT = -2147467259; // 0x80004005
pub const E_POINTER: HRESULT = -2147467261; // 0x80004003

// Data flow direction
pub const eRender: i32 = 0;
pub const eCapture: i32 = 1;
pub const eAll: i32 = 2;

// Device role
pub const eConsole: i32 = 0;
pub const eMultimedia: i32 = 1;
pub const eCommunications: i32 = 2;

// GUIDs for device enumerator
pub const CLSID_MMDeviceEnumerator: GUID = GUID {
    Data1: 0xBCDE0395,
    Data2: 0xE52F,
    Data3: 0x467C,
    Data4: [0x8E, 0x3D, 0xC4, 0x57, 0x92, 0x91, 0x69, 0x2E],
};

pub const IID_IMMDeviceEnumerator: GUID = GUID {
    Data1: 0xA95664D2,
    Data2: 0x9614,
    Data3: 0x4F35,
    Data4: [0xA7, 0x46, 0xDE, 0x8D, 0xB6, 0x36, 0x17, 0xE6],
};

pub const IID_IAudioClient: GUID = GUID {
    Data1: 0x1CB9AD4C,
    Data2: 0xDBFA,
    Data3: 0x4C32,
    Data4: [0xB1, 0x78, 0xC2, 0xF5, 0x68, 0xA7, 0x03, 0xB2],
};

pub const IID_IAudioCaptureClient: GUID = GUID {
    Data1: 0xC8ADBD64,
    Data2: 0xE71E,
    Data3: 0x48A0,
    Data4: [0xA4, 0xDE, 0x18, 0x5C, 0x39, 0x5C, 0xD3, 0x17],
};

// KSDATAFORMAT_SUBTYPE_IEEE_FLOAT
pub const KSDATAFORMAT_SUBTYPE_IEEE_FLOAT: GUID = GUID {
    Data1: 0x00000003,
    Data2: 0x0000,
    Data3: 0x0010,
    Data4: [0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71],
};

// IMMDeviceEnumerator interface
#[repr(C)]
pub struct IMMDeviceEnumerator {
    pub lpVtbl: *const IMMDeviceEnumeratorVtbl,
}

#[repr(C)]
pub struct IMMDeviceEnumeratorVtbl {
    pub parent: IUnknownVtbl,
    pub EnumAudioEndpoints: unsafe extern "system" fn(
        This: *mut IMMDeviceEnumerator,
        dataFlow: i32,
        dwStateMask: DWORD,
        ppDevices: *mut *mut c_void,
    ) -> HRESULT,
    pub GetDefaultAudioEndpoint: unsafe extern "system" fn(
        This: *mut IMMDeviceEnumerator,
        dataFlow: i32,
        role: i32,
        ppEndpoint: *mut *mut IMMDevice,
    ) -> HRESULT,
    pub GetDevice: unsafe extern "system" fn(
        This: *mut IMMDeviceEnumerator,
        pwstrId: LPCWSTR,
        ppDevice: *mut *mut IMMDevice,
    ) -> HRESULT,
    pub RegisterEndpointNotificationCallback: unsafe extern "system" fn(
        This: *mut IMMDeviceEnumerator,
        pClient: *mut c_void,
    ) -> HRESULT,
    pub UnregisterEndpointNotificationCallback: unsafe extern "system" fn(
        This: *mut IMMDeviceEnumerator,
        pClient: *mut c_void,
    ) -> HRESULT,
}

// IMMDevice interface
#[repr(C)]
pub struct IMMDevice {
    pub lpVtbl: *const IMMDeviceVtbl,
}

#[repr(C)]
pub struct IMMDeviceVtbl {
    pub parent: IUnknownVtbl,
    pub Activate: unsafe extern "system" fn(
        This: *mut IMMDevice,
        iid: *const GUID,
        dwClsCtx: DWORD,
        pActivationParams: *mut c_void,
        ppInterface: *mut *mut c_void,
    ) -> HRESULT,
    pub OpenPropertyStore: unsafe extern "system" fn(
        This: *mut IMMDevice,
        stgmAccess: DWORD,
        ppProperties: *mut *mut c_void,
    ) -> HRESULT,
    pub GetId: unsafe extern "system" fn(
        This: *mut IMMDevice,
        ppstrId: *mut LPWSTR,
    ) -> HRESULT,
    pub GetState: unsafe extern "system" fn(
        This: *mut IMMDevice,
        pdwState: *mut DWORD,
    ) -> HRESULT,
}

// IAudioClient interface
#[repr(C)]
pub struct IAudioClient {
    pub lpVtbl: *const IAudioClientVtbl,
}

#[repr(C)]
pub struct IAudioClientVtbl {
    pub parent: IUnknownVtbl,
    pub Initialize: unsafe extern "system" fn(
        This: *mut IAudioClient,
        ShareMode: i32,
        StreamFlags: DWORD,
        hnsBufferDuration: REFERENCE_TIME,
        hnsPeriodicity: REFERENCE_TIME,
        pFormat: *const WAVEFORMATEX,
        AudioSessionGuid: *const GUID,
    ) -> HRESULT,
    pub GetBufferSize: unsafe extern "system" fn(
        This: *mut IAudioClient,
        pNumBufferFrames: *mut UINT32,
    ) -> HRESULT,
    pub GetStreamLatency: unsafe extern "system" fn(
        This: *mut IAudioClient,
        phnsLatency: *mut REFERENCE_TIME,
    ) -> HRESULT,
    pub GetCurrentPadding: unsafe extern "system" fn(
        This: *mut IAudioClient,
        pNumPaddingFrames: *mut UINT32,
    ) -> HRESULT,
    pub IsFormatSupported: unsafe extern "system" fn(
        This: *mut IAudioClient,
        ShareMode: i32,
        pFormat: *const WAVEFORMATEX,
        ppClosestMatch: *mut *mut WAVEFORMATEX,
    ) -> HRESULT,
    pub GetMixFormat: unsafe extern "system" fn(
        This: *mut IAudioClient,
        ppDeviceFormat: *mut *mut WAVEFORMATEX,
    ) -> HRESULT,
    pub GetDevicePeriod: unsafe extern "system" fn(
        This: *mut IAudioClient,
        phnsDefaultDevicePeriod: *mut REFERENCE_TIME,
        phnsMinimumDevicePeriod: *mut REFERENCE_TIME,
    ) -> HRESULT,
    pub Start: unsafe extern "system" fn(
        This: *mut IAudioClient,
    ) -> HRESULT,
    pub Stop: unsafe extern "system" fn(
        This: *mut IAudioClient,
    ) -> HRESULT,
    pub Reset: unsafe extern "system" fn(
        This: *mut IAudioClient,
    ) -> HRESULT,
    pub SetEventHandle: unsafe extern "system" fn(
        This: *mut IAudioClient,
        eventHandle: *mut c_void,
    ) -> HRESULT,
    pub GetService: unsafe extern "system" fn(
        This: *mut IAudioClient,
        riid: *const GUID,
        ppv: *mut *mut c_void,
    ) -> HRESULT,
}

// IAudioCaptureClient interface
#[repr(C)]
pub struct IAudioCaptureClient {
    pub lpVtbl: *const IAudioCaptureClientVtbl,
}

#[repr(C)]
pub struct IAudioCaptureClientVtbl {
    pub parent: IUnknownVtbl,
    pub GetBuffer: unsafe extern "system" fn(
        This: *mut IAudioCaptureClient,
        ppData: *mut *mut u8,
        pNumFramesToRead: *mut UINT32,
        pdwFlags: *mut DWORD,
        pu64DevicePosition: *mut u64,
        pu64QPCPosition: *mut u64,
    ) -> HRESULT,
    pub ReleaseBuffer: unsafe extern "system" fn(
        This: *mut IAudioCaptureClient,
        NumFramesRead: UINT32,
    ) -> HRESULT,
    pub GetNextPacketSize: unsafe extern "system" fn(
        This: *mut IAudioCaptureClient,
        pNumFramesInNextPacket: *mut UINT32,
    ) -> HRESULT,
}

// COM functions
extern "system" {
    pub fn CoInitializeEx(pvReserved: *mut c_void, dwCoInit: DWORD) -> HRESULT;
    pub fn CoUninitialize();
    pub fn CoCreateInstance(
        rclsid: *const GUID,
        pUnkOuter: *mut IUnknown,
        dwClsContext: DWORD,
        riid: *const GUID,
        ppv: *mut *mut c_void,
    ) -> HRESULT;
    pub fn CoTaskMemFree(pv: *mut c_void);
}

// COM constants
pub const COINIT_MULTITHREADED: DWORD = 0x0;
pub const CLSCTX_ALL: DWORD = 0x17;
pub const CLSCTX_INPROC_SERVER: DWORD = 0x1;

// Device state
pub const DEVICE_STATE_ACTIVE: DWORD = 0x00000001;

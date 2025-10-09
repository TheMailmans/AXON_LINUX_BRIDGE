use anyhow::Result;
use tracing::info;

// Platform-specific implementations
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "linux")]
pub mod linux;

/// Capture mode
#[derive(Debug, Clone)]
pub enum CaptureMode {
    Desktop,
    Window(String),
    Region { x: i32, y: i32, width: u32, height: u32 },
}

/// Capture configuration
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub mode: CaptureMode,
    pub frame_rate: u32,
    pub quality: u32,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            mode: CaptureMode::Desktop,
            frame_rate: 30,
            quality: 80,
        }
    }
}

/// Capture Manager
/// 
/// Manages screen capture across platforms
pub struct CaptureManager {
    config: CaptureConfig,
    #[allow(dead_code)]
    platform_capturer: Box<dyn PlatformCapturer>,
}

impl CaptureManager {
    pub fn new() -> Result<Self> {
        Self::with_config(CaptureConfig::default())
    }
    
    pub fn with_config(config: CaptureConfig) -> Result<Self> {
        #[cfg(target_os = "macos")]
        info!("Initializing capture manager for platform: macOS");
        #[cfg(target_os = "windows")]
        info!("Initializing capture manager for platform: Windows");
        #[cfg(target_os = "linux")]
        info!("Initializing capture manager for platform: Linux");
        
        let platform_capturer = create_platform_capturer()?;
        
        Ok(Self {
            config,
            platform_capturer,
        })
    }
    
    pub fn start(&mut self) -> Result<()> {
        info!("Starting capture with config: {:?}", self.config);
        self.platform_capturer.start(&self.config)?;
        Ok(())
    }
    
    pub fn stop(&mut self) -> Result<()> {
        info!("Stopping capture");
        self.platform_capturer.stop()?;
        Ok(())
    }
    
    pub fn get_frame(&mut self) -> Result<Vec<u8>> {
        self.platform_capturer.get_frame()
    }
}

/// Platform-specific capturer trait
trait PlatformCapturer: Send + Sync {
    fn start(&mut self, config: &CaptureConfig) -> Result<()>;
    fn stop(&mut self) -> Result<()>;
    fn get_frame(&mut self) -> Result<Vec<u8>>;
}

/// Create platform-specific capturer
fn create_platform_capturer() -> Result<Box<dyn PlatformCapturer>> {
    #[cfg(target_os = "windows")]
    {
        Ok(Box::new(windows::WindowsCapturer::new()?))
    }
    
    #[cfg(target_os = "macos")]
    {
        Ok(Box::new(macos::MacOSCapturer::new()?))
    }
    
    #[cfg(target_os = "linux")]
    {
        Ok(Box::new(linux::LinuxCapturer::new()?))
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err(anyhow::anyhow!("Unsupported platform"))
    }
}

// Platform-specific implementations

#[cfg(target_os = "windows")]
impl PlatformCapturer for windows::WindowsCapturer {
    fn start(&mut self, config: &CaptureConfig) -> Result<()> {
        windows::WindowsCapturer::start(self, config)
    }
    
    fn stop(&mut self) -> Result<()> {
        windows::WindowsCapturer::stop(self)
    }
    
    fn get_frame(&mut self) -> Result<Vec<u8>> {
        windows::WindowsCapturer::get_frame(self)
    }
}

#[cfg(target_os = "macos")]
impl PlatformCapturer for macos::MacOSCapturer {
    fn start(&mut self, config: &CaptureConfig) -> Result<()> {
        macos::MacOSCapturer::start(self, config)
    }
    
    fn stop(&mut self) -> Result<()> {
        macos::MacOSCapturer::stop(self)
    }
    
    fn get_frame(&mut self) -> Result<Vec<u8>> {
        macos::MacOSCapturer::get_frame(self)
    }
}

#[cfg(target_os = "linux")]
impl PlatformCapturer for linux::LinuxCapturer {
    fn start(&mut self, config: &CaptureConfig) -> Result<()> {
        linux::LinuxCapturer::start(self, config)
    }
    
    fn stop(&mut self) -> Result<()> {
        linux::LinuxCapturer::stop(self)
    }
    
    fn get_frame(&mut self) -> Result<Vec<u8>> {
        linux::LinuxCapturer::get_frame(self)
    }
}

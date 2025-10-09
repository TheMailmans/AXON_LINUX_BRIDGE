/*!
 * Windows Screen Capture Implementation
 * 
 * Full implementation using Windows.Graphics.Capture API (Windows 10 1903+)
 * with Direct3D11 for hardware-accelerated frame capture.
 * 
 * See windows_impl.rs for the complete implementation.
 * This file re-exports the implementation for compatibility.
 */

mod windows_impl;

pub use windows_impl::WindowsCapturer;

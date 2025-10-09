/// Input injection module

// macOS implementation
#[cfg(target_os = "macos")]
pub mod macos_keys;

#[cfg(target_os = "macos")]
pub mod keyboard;

#[cfg(target_os = "macos")]
pub mod mouse;

#[cfg(target_os = "macos")]
pub use keyboard::{inject_key_press, type_string};

#[cfg(target_os = "macos")]
pub use mouse::{inject_mouse_move, inject_mouse_click};

// Linux implementation
#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "linux")]
pub use linux::{inject_key_press, type_string, inject_mouse_move, inject_mouse_click};

// Windows stub (not implemented yet)
#[cfg(target_os = "windows")]
pub fn inject_key_press(_key: &str, _modifiers: &[String]) -> anyhow::Result<()> {
    anyhow::bail!("Input injection not implemented for Windows yet")
}

#[cfg(target_os = "windows")]
pub fn type_string(_text: &str) -> anyhow::Result<()> {
    anyhow::bail!("Input injection not implemented for Windows yet")
}

#[cfg(target_os = "windows")]
pub fn inject_mouse_move(_x: i32, _y: i32) -> anyhow::Result<()> {
    anyhow::bail!("Input injection not implemented for Windows yet")
}

#[cfg(target_os = "windows")]
pub fn inject_mouse_click(_x: i32, _y: i32, _button: &str) -> anyhow::Result<()> {
    anyhow::bail!("Input injection not implemented for Windows yet")
}

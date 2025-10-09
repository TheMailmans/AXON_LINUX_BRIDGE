/// macOS keyboard injection implementation using Core Graphics

use core_graphics::event::{CGEvent, CGEventTapLocation, CGEventType};
use std::thread;
use std::time::Duration;
use anyhow::Result;
use tracing::info;

use super::macos_keys::{key_name_to_code, combine_modifiers};

/// Inject a key press with modifiers
pub fn inject_key_press(key: &str, modifiers: &[String]) -> Result<()> {
    info!("Injecting key press: key={}, modifiers={:?}", key, modifiers);
    
    // Get key code
    let key_code = key_name_to_code(key)
        .ok_or_else(|| anyhow::anyhow!("Unknown key: {}", key))?;
    
    info!("Mapped key '{}' to code: {}", key, key_code);
    
    // Get modifier flags
    let modifier_flags = combine_modifiers(modifiers);
    
    // Create event source
    let event_source = core_graphics::event_source::CGEventSource::new(
        core_graphics::event_source::CGEventSourceStateID::HIDSystemState
    ).map_err(|_| anyhow::anyhow!("Failed to create event source"))?;
    
    // Create key down event
    let key_down = CGEvent::new_keyboard_event(event_source.clone(), key_code, true)
        .map_err(|_| anyhow::anyhow!("Failed to create key down event"))?;
    
    // Set modifier flags
    key_down.set_flags(modifier_flags);
    
    // Post key down
    key_down.post(CGEventTapLocation::HID);
    info!("Posted key down event");
    
    // Small delay between key down and key up
    thread::sleep(Duration::from_millis(10));
    
    // Create key up event
    let key_up = CGEvent::new_keyboard_event(event_source, key_code, false)
        .map_err(|_| anyhow::anyhow!("Failed to create key up event"))?;
    
    // Set modifier flags for key up
    key_up.set_flags(modifier_flags);
    
    // Post key up
    key_up.post(CGEventTapLocation::HID);
    info!("Posted key up event");
    
    // Small delay after injection
    thread::sleep(Duration::from_millis(10));
    
    info!("Key press injected successfully: {}", key);
    Ok(())
}

/// Type a string by injecting individual characters
pub fn type_string(text: &str) -> Result<()> {
    info!("Typing string: {} characters", text.len());
    
    for ch in text.chars() {
        inject_character(ch)?;
        thread::sleep(Duration::from_millis(20));
    }
    
    info!("String typed successfully");
    Ok(())
}

/// Inject a single character
fn inject_character(ch: char) -> Result<()> {
    let key_str = ch.to_string();
    let modifiers = if ch.is_uppercase() {
        vec!["shift".to_string()]
    } else {
        vec![]
    };
    
    inject_key_press(&key_str.to_lowercase(), &modifiers)
}

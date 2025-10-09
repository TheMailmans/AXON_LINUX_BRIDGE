/// macOS mouse injection implementation using Core Graphics

use core_graphics::event::{CGEvent, CGEventTapLocation, CGEventType, CGMouseButton};
use core_graphics::geometry::CGPoint;
use std::thread;
use std::time::Duration;
use anyhow::Result;
use tracing::info;

/// Inject mouse move to absolute position
pub fn inject_mouse_move(x: i32, y: i32) -> Result<()> {
    info!("Injecting mouse move: x={}, y={}", x, y);
    
    let point = CGPoint::new(x as f64, y as f64);
    
    let event_source = core_graphics::event_source::CGEventSource::new(
        core_graphics::event_source::CGEventSourceStateID::HIDSystemState
    ).map_err(|_| anyhow::anyhow!("Failed to create event source"))?;
    
    let move_event = CGEvent::new_mouse_event(
        event_source,
        CGEventType::MouseMoved,
        point,
        CGMouseButton::Left,
    ).map_err(|_| anyhow::anyhow!("Failed to create mouse move event"))?;
    
    move_event.post(CGEventTapLocation::HID);
    
    thread::sleep(Duration::from_millis(10));
    
    info!("Mouse moved to ({}, {})", x, y);
    Ok(())
}

/// Inject mouse click at position
pub fn inject_mouse_click(x: i32, y: i32, button: &str) -> Result<()> {
    info!("Injecting mouse click: x={}, y={}, button={}", x, y, button);
    
    let point = CGPoint::new(x as f64, y as f64);
    let cg_button = match button.to_lowercase().as_str() {
        "left" => CGMouseButton::Left,
        "right" => CGMouseButton::Right,
        "middle" => CGMouseButton::Center,
        _ => CGMouseButton::Left,
    };
    
    let event_source = core_graphics::event_source::CGEventSource::new(
        core_graphics::event_source::CGEventSourceStateID::HIDSystemState
    ).map_err(|_| anyhow::anyhow!("Failed to create event source"))?;
    
    // Mouse down
    let down_type = match cg_button {
        CGMouseButton::Left => CGEventType::LeftMouseDown,
        CGMouseButton::Right => CGEventType::RightMouseDown,
        CGMouseButton::Center => CGEventType::OtherMouseDown,
    };
    
    let down_event = CGEvent::new_mouse_event(
        event_source.clone(),
        down_type,
        point,
        cg_button,
    ).map_err(|_| anyhow::anyhow!("Failed to create mouse down event"))?;
    
    down_event.post(CGEventTapLocation::HID);
    
    thread::sleep(Duration::from_millis(50));
    
    // Mouse up
    let up_type = match cg_button {
        CGMouseButton::Left => CGEventType::LeftMouseUp,
        CGMouseButton::Right => CGEventType::RightMouseUp,
        CGMouseButton::Center => CGEventType::OtherMouseUp,
    };
    
    let up_event = CGEvent::new_mouse_event(
        event_source,
        up_type,
        point,
        cg_button,
    ).map_err(|_| anyhow::anyhow!("Failed to create mouse up event"))?;
    
    up_event.post(CGEventTapLocation::HID);
    
    thread::sleep(Duration::from_millis(10));
    
    info!("Mouse click completed at ({}, {})", x, y);
    Ok(())
}

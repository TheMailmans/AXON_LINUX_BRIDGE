/// macOS keyboard key codes and modifier mappings
/// Reference: https://developer.apple.com/documentation/coregraphics/cgkeycode

use core_graphics::event::{CGKeyCode, CGEventFlags};

/// Map common key names to macOS key codes
pub fn key_name_to_code(key: &str) -> Option<CGKeyCode> {
    match key.to_lowercase().as_str() {
        // Letters
        "a" => Some(0x00),
        "b" => Some(0x0B),
        "c" => Some(0x08),
        "d" => Some(0x02),
        "e" => Some(0x0E),
        "f" => Some(0x03),
        "g" => Some(0x05),
        "h" => Some(0x04),
        "i" => Some(0x22),
        "j" => Some(0x26),
        "k" => Some(0x28),
        "l" => Some(0x25),
        "m" => Some(0x2E),
        "n" => Some(0x2D),
        "o" => Some(0x1F),
        "p" => Some(0x23),
        "q" => Some(0x0C),
        "r" => Some(0x0F),
        "s" => Some(0x01),
        "t" => Some(0x11),
        "u" => Some(0x20),
        "v" => Some(0x09),
        "w" => Some(0x0D),
        "x" => Some(0x07),
        "y" => Some(0x10),
        "z" => Some(0x06),
        
        // Numbers
        "0" => Some(0x1D),
        "1" => Some(0x12),
        "2" => Some(0x13),
        "3" => Some(0x14),
        "4" => Some(0x15),
        "5" => Some(0x17),
        "6" => Some(0x16),
        "7" => Some(0x1A),
        "8" => Some(0x1C),
        "9" => Some(0x19),
        
        // Special keys
        "return" | "enter" => Some(0x24),
        "tab" => Some(0x30),
        "space" => Some(0x31),
        "delete" | "backspace" => Some(0x33),
        "escape" | "esc" => Some(0x35),
        "command" | "cmd" => Some(0x37),
        "shift" => Some(0x38),
        "capslock" => Some(0x39),
        "option" | "alt" => Some(0x3A),
        "control" | "ctrl" => Some(0x3B),
        "rightshift" => Some(0x3C),
        "rightoption" => Some(0x3D),
        "rightcontrol" => Some(0x3E),
        "function" | "fn" => Some(0x3F),
        
        // Arrow keys
        "left" | "leftarrow" => Some(0x7B),
        "right" | "rightarrow" => Some(0x7C),
        "down" | "downarrow" => Some(0x7D),
        "up" | "uparrow" => Some(0x7E),
        
        // Function keys
        "f1" => Some(0x7A),
        "f2" => Some(0x78),
        "f3" => Some(0x63),
        "f4" => Some(0x76),
        "f5" => Some(0x60),
        "f6" => Some(0x61),
        "f7" => Some(0x62),
        "f8" => Some(0x64),
        "f9" => Some(0x65),
        "f10" => Some(0x6D),
        "f11" => Some(0x67),
        "f12" => Some(0x6F),
        
        // Other keys
        "home" => Some(0x73),
        "end" => Some(0x77),
        "pageup" => Some(0x74),
        "pagedown" => Some(0x79),
        "forwarddelete" => Some(0x75),
        
        _ => None,
    }
}

/// Map modifier names to CGEventFlags
pub fn modifier_to_flag(modifier: &str) -> Option<CGEventFlags> {
    match modifier.to_lowercase().as_str() {
        "command" | "cmd" => Some(CGEventFlags::CGEventFlagCommand),
        "shift" => Some(CGEventFlags::CGEventFlagShift),
        "option" | "alt" => Some(CGEventFlags::CGEventFlagAlternate),
        "control" | "ctrl" => Some(CGEventFlags::CGEventFlagControl),
        _ => None,
    }
}

/// Combine multiple modifiers into a single flag
pub fn combine_modifiers(modifiers: &[String]) -> CGEventFlags {
    let mut combined = CGEventFlags::empty();
    
    for modifier in modifiers {
        if let Some(flag) = modifier_to_flag(modifier) {
            combined = combined | flag;
        }
    }
    
    combined
}

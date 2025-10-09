/*!
 * Shortcut Normalizer
 * 
 * Converts AT-SPI keyboard shortcuts to normalized format.
 */

use tracing::debug;

pub struct ShortcutNormalizer;

impl ShortcutNormalizer {
    pub fn new() -> Self {
        Self
    }
    
    /// Normalize AT-SPI shortcut string to key list
    /// 
    /// Examples:
    /// - "g" → ["g"]
    /// - "Ctrl+L" → ["ctrl", "l"]
    /// - "<Control>l" → ["ctrl", "l"]
    /// - "Ctrl+Shift+N" → ["ctrl", "shift", "n"]
    pub fn normalize(&self, shortcut: &str) -> Vec<String> {
        if shortcut.is_empty() {
            return Vec::new();
        }
        
        // Remove angle brackets: <Control>l → Control l
        let clean = shortcut.replace('<', "").replace('>', " ");
        
        // Split on + or whitespace
        let parts: Vec<&str> = clean.split(&['+', ' '][..])
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        
        let mut normalized = Vec::new();
        
        for part in parts {
            let lower = part.to_lowercase();
            
            // Map modifier keys
            let key = match lower.as_str() {
                "control" | "ctrl" => "ctrl",
                "alt" | "meta" => "alt",
                "shift" => "shift",
                "super" | "cmd" | "command" | "win" | "windows" => "command",
                "del" | "delete" => "delete",
                "esc" | "escape" => "esc",
                "return" | "enter" => "return",
                "tab" => "tab",
                "space" => "space",
                "backspace" => "backspace",
                _ => &lower, // Keep as-is (letters, numbers, function keys)
            };
            
            normalized.push(key.to_string());
        }
        
        debug!("Normalized '{}' → {:?}", shortcut, normalized);
        normalized
    }
    
    /// Generate command string for execution
    /// 
    /// Returns a string like "Ctrl+L" for display/logging
    pub fn to_command(&self, keys: &[String]) -> String {
        if keys.is_empty() {
            return String::new();
        }
        
        keys.join("+")
    }
    
    /// Check if shortcut is single-key (requires menu open)
    pub fn is_single_key(&self, keys: &[String]) -> bool {
        keys.len() == 1
    }
}

impl Default for ShortcutNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_normalize_single() {
        let norm = ShortcutNormalizer::new();
        assert_eq!(norm.normalize("g"), vec!["g"]);
    }
    
    #[test]
    fn test_normalize_ctrl_plus() {
        let norm = ShortcutNormalizer::new();
        assert_eq!(norm.normalize("Ctrl+L"), vec!["ctrl", "l"]);
    }
    
    #[test]
    fn test_normalize_angle_brackets() {
        let norm = ShortcutNormalizer::new();
        assert_eq!(norm.normalize("<Control>l"), vec!["ctrl", "l"]);
    }
    
    #[test]
    fn test_normalize_multiple_modifiers() {
        let norm = ShortcutNormalizer::new();
        assert_eq!(norm.normalize("Ctrl+Shift+N"), vec!["ctrl", "shift", "n"]);
    }
    
    #[test]
    fn test_to_command() {
        let norm = ShortcutNormalizer::new();
        let keys = vec!["ctrl".to_string(), "l".to_string()];
        assert_eq!(norm.to_command(&keys), "ctrl+l");
    }
    
    #[test]
    fn test_is_single_key() {
        let norm = ShortcutNormalizer::new();
        assert!(norm.is_single_key(&vec!["g".to_string()]));
        assert!(!norm.is_single_key(&vec!["ctrl".to_string(), "l".to_string()]));
    }
}

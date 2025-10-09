/*!
 * Accessibility Module
 * 
 * Captures and parses AT-SPI accessibility trees on Linux to extract keyboard shortcuts.
 */

pub mod parser;
pub mod normalizer;
pub mod capture;

pub use parser::A11yParser;
pub use normalizer::ShortcutNormalizer;
pub use capture::capture_accessibility_tree;

use std::collections::HashMap;

/// Shortcut discovery result
#[derive(Debug, Clone)]
pub struct ShortcutInfo {
    pub name: String,
    pub raw_shortcut: String,
    pub normalized_keys: Vec<String>,
    pub command: String,
}

/// Main A11y interface
pub struct A11yModule {
    parser: A11yParser,
    normalizer: ShortcutNormalizer,
}

impl A11yModule {
    pub fn new() -> Self {
        Self {
            parser: A11yParser::new(),
            normalizer: ShortcutNormalizer::new(),
        }
    }
    
    /// Capture, parse, and extract shortcuts in one call
    pub fn discover_shortcuts(&self) -> anyhow::Result<Vec<ShortcutInfo>> {
        // Capture a11y tree
        let tree_xml = capture::capture_accessibility_tree()?;
        
        // Parse for shortcuts
        let raw_shortcuts = self.parser.parse_and_extract(&tree_xml)?;
        
        // Normalize each shortcut
        let mut shortcuts = Vec::new();
        for (name, raw_shortcut) in raw_shortcuts {
            let keys = self.normalizer.normalize(&raw_shortcut);
            let command = self.normalizer.to_command(&keys);
            
            shortcuts.push(ShortcutInfo {
                name,
                raw_shortcut,
                normalized_keys: keys,
                command,
            });
        }
        
        Ok(shortcuts)
    }
}

impl Default for A11yModule {
    fn default() -> Self {
        Self::new()
    }
}

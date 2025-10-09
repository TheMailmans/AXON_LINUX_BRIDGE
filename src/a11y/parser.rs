/*!
 * AT-SPI XML Parser
 * 
 * Parses Linux AT-SPI accessibility trees to extract keyboard shortcuts.
 */

use anyhow::{Result, anyhow};
use std::collections::HashMap;
use tracing::{debug, warn};

const ATTR_NS: &str = "https://accessibility.ubuntu.example.org/ns/attributes";

pub struct A11yParser;

impl A11yParser {
    pub fn new() -> Self {
        Self
    }
    
    /// Parse AT-SPI XML and extract shortcuts
    pub fn parse_and_extract(&self, xml: &str) -> Result<HashMap<String, String>> {
        // Try XML parsing first
        match self.parse_xml(xml) {
            Ok(shortcuts) => {
                debug!("Parsed {} shortcuts via XML", shortcuts.len());
                Ok(shortcuts)
            }
            Err(e) => {
                warn!("XML parsing failed: {}, falling back to regex", e);
                // Fallback to regex
                self.parse_regex(xml)
            }
        }
    }
    
    /// Parse using XML library
    fn parse_xml(&self, xml: &str) -> Result<HashMap<String, String>> {
        use quick_xml::Reader;
        use quick_xml::events::Event;
        
        let mut shortcuts = HashMap::new();
        let mut reader = Reader::from_str(xml);
        reader.trim_text(true);
        
        let mut buf = Vec::new();
        
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(e)) | Ok(Event::Start(e)) => {
                    // Get name attribute
                    let name = e.attributes()
                        .filter_map(|a| a.ok())
                        .find(|a| a.key.as_ref() == b"name")
                        .and_then(|a| String::from_utf8(a.value.to_vec()).ok());
                    
                    // Get keyshortcuts attribute (with or without namespace)
                    let keyshortcut = e.attributes()
                        .filter_map(|a| a.ok())
                        .find(|a| {
                            let key = a.key.as_ref();
                            key == b"attr:keyshortcuts" || 
                            key.ends_with(b":keyshortcuts") ||
                            key == b"keyshortcuts"
                        })
                        .and_then(|a| String::from_utf8(a.value.to_vec()).ok());
                    
                    // If both present, add to map
                    if let (Some(n), Some(k)) = (name, keyshortcut) {
                        shortcuts.insert(n, k);
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(anyhow!("XML parse error: {}", e)),
                _ => {}
            }
            buf.clear();
        }
        
        if shortcuts.is_empty() {
            return Err(anyhow!("No shortcuts found in XML"));
        }
        
        Ok(shortcuts)
    }
    
    /// Parse using regex fallback (for malformed XML)
    fn parse_regex(&self, xml: &str) -> Result<HashMap<String, String>> {
        use regex::Regex;
        
        let mut shortcuts = HashMap::new();
        
        // Pattern: name="..." ... attr:keyshortcuts="..."
        let re = Regex::new(r#"name="([^"]+)"[^>]*(?:attr:)?keyshortcuts="([^"]+)""#)
            .map_err(|e| anyhow!("Regex error: {}", e))?;
        
        for cap in re.captures_iter(xml) {
            if let (Some(name), Some(shortcut)) = (cap.get(1), cap.get(2)) {
                shortcuts.insert(
                    name.as_str().to_string(),
                    shortcut.as_str().to_string()
                );
            }
        }
        
        // Also try reverse order: keyshortcuts first, then name
        let re2 = Regex::new(r#"(?:attr:)?keyshortcuts="([^"]+)"[^>]*name="([^"]+)""#)
            .map_err(|e| anyhow!("Regex error: {}", e))?;
        
        for cap in re2.captures_iter(xml) {
            if let (Some(shortcut), Some(name)) = (cap.get(1), cap.get(2)) {
                shortcuts.insert(
                    name.as_str().to_string(),
                    shortcut.as_str().to_string()
                );
            }
        }
        
        if shortcuts.is_empty() {
            return Err(anyhow!("No shortcuts found via regex"));
        }
        
        debug!("Regex fallback found {} shortcuts", shortcuts.len());
        Ok(shortcuts)
    }
}

impl Default for A11yParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_simple() {
        let xml = r#"
            <desktop-frame xmlns:attr="https://accessibility.ubuntu.example.org/ns/attributes">
                <menuitem name="Settings" attr:keyshortcuts="g"/>
            </desktop-frame>
        "#;
        
        let parser = A11yParser::new();
        let shortcuts = parser.parse_and_extract(xml).unwrap();
        
        assert_eq!(shortcuts.get("Settings"), Some(&"g".to_string()));
    }
    
    #[test]
    fn test_parse_multiple() {
        let xml = r#"
            <desktop-frame xmlns:attr="https://accessibility.ubuntu.example.org/ns/attributes">
                <menuitem name="Settings" attr:keyshortcuts="g"/>
                <entry name="Address bar" attr:keyshortcuts="Ctrl+L"/>
            </desktop-frame>
        "#;
        
        let parser = A11yParser::new();
        let shortcuts = parser.parse_and_extract(xml).unwrap();
        
        assert_eq!(shortcuts.len(), 2);
        assert_eq!(shortcuts.get("Settings"), Some(&"g".to_string()));
        assert_eq!(shortcuts.get("Address bar"), Some(&"Ctrl+L".to_string()));
    }
}

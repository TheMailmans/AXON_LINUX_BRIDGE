// Universal Application Launcher
// Follows FreeDesktop.org Desktop Entry Specification
// https://specifications.freedesktop.org/desktop-entry-spec/latest/

use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::fs;
use std::process::Command;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use tokio::task;
use tracing::{info, warn, debug, error};

/// Represents a desktop application from a .desktop file
#[derive(Debug, Clone)]
pub struct DesktopApp {
    /// Desktop file ID (e.g., "firefox.desktop")
    pub id: String,
    /// Application display name (e.g., "Firefox Web Browser")
    pub name: String,
    /// Exec command line (e.g., "firefox %u")
    pub exec: String,
    /// Keywords for search (e.g., ["browser", "web", "internet"])
    pub keywords: Vec<String>,
    /// Generic application category (e.g., "Web Browser")
    pub generic_name: String,
    /// Full path to .desktop file
    pub path: PathBuf,
}

/// Index of all installed desktop applications
pub struct AppIndex {
    apps: Vec<DesktopApp>,
    last_updated: SystemTime,
}

impl AppIndex {
    /// Create new app index by scanning standard directories
    pub fn new() -> Self {
        let mut index = Self {
            apps: Vec::new(),
            last_updated: SystemTime::now(),
        };
        index.scan_applications();
        index
    }
    
    /// Scan all standard application directories
    fn scan_applications(&mut self) {
        info!("🔍 Scanning for installed applications...");
        
        // Standard FreeDesktop.org directories
        self.scan_directory("/usr/share/applications");
        self.scan_directory("/usr/local/share/applications");
        
        // User-specific directory
        if let Ok(home) = std::env::var("HOME") {
            self.scan_directory(&format!("{}/.local/share/applications", home));
        }
        
        // Flatpak applications
        if let Ok(home) = std::env::var("HOME") {
            self.scan_directory(&format!("{}/.local/share/flatpak/exports/share/applications", home));
        }
        
        // Snap applications
        self.scan_directory("/var/lib/snapd/desktop/applications");
        
        info!("✅ Found {} applications", self.apps.len());
    }
    
    /// Scan a single directory for .desktop files
    fn scan_directory(&mut self, dir_path: &str) {
        let path = Path::new(dir_path);
        
        if !path.exists() {
            debug!("Directory does not exist: {}", dir_path);
            return;
        }
        
        let entries = match fs::read_dir(path) {
            Ok(e) => e,
            Err(e) => {
                warn!("Failed to read directory {}: {}", dir_path, e);
                return;
            }
        };
        
        for entry in entries.flatten() {
            let path = entry.path();
            
            // Only process .desktop files
            if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                if let Some(app) = Self::parse_desktop_file(&path) {
                    debug!("Found app: {} ({})", app.name, app.id);
                    self.apps.push(app);
                }
            }
        }
    }
    
    /// Parse a single .desktop file
    fn parse_desktop_file(path: &Path) -> Option<DesktopApp> {
        use configparser::ini::Ini;
        
        let mut conf = Ini::new();
        if conf.load(path.to_str()?).is_err() {
            debug!("Failed to parse {}", path.display());
            return None;
        }
        
        // Get Desktop Entry section
        let section = "Desktop Entry";
        
        // Only process Type=Application entries
        if conf.get(section, "Type")? != "Application" {
            return None;
        }
        
        // Skip hidden entries (NoDisplay=true)
        if conf.get(section, "NoDisplay").as_deref() == Some("true") {
            return None;
        }
        
        // Skip entries without Exec command
        let exec = conf.get(section, "Exec")?;
        
        let id = path.file_name()?.to_string_lossy().to_string();
        let name = conf.get(section, "Name")?;
        let generic_name = conf.get(section, "GenericName").unwrap_or_default();
        
        // Parse semicolon-separated keywords
        let keywords = conf.get(section, "Keywords")
            .map(|k| k.split(';')
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect())
            .unwrap_or_default();
        
        Some(DesktopApp {
            id,
            name,
            exec,
            keywords,
            generic_name,
            path: path.to_path_buf(),
        })
    }
    
    /// Find best matching app using fuzzy matching
    /// 
    /// Matches against: Name, ID, GenericName, Keywords
    /// Returns None if no good match found (score < 30)
    pub fn find_app(&self, query: &str) -> Option<&DesktopApp> {
        let matcher = SkimMatcherV2::default();
        let query_lower = query.to_lowercase();
        
        let mut best_score = 0i64;
        let mut best_app = None;
        
        for app in &self.apps {
            // Try matching against multiple fields
            let mut scores = vec![
                matcher.fuzzy_match(&app.name.to_lowercase(), &query_lower),
                matcher.fuzzy_match(&app.id.to_lowercase(), &query_lower),
                matcher.fuzzy_match(&app.generic_name.to_lowercase(), &query_lower),
            ];
            
            // Check keywords too
            for keyword in &app.keywords {
                scores.push(matcher.fuzzy_match(&keyword.to_lowercase(), &query_lower));
            }
            
            let max_score = scores.into_iter()
                .filter_map(|s| s)
                .max()
                .unwrap_or(0);
            
            if max_score > best_score {
                best_score = max_score;
                best_app = Some(app);
            }
        }
        
        // Only return if confidence is high enough
        if best_score > 30 {
            info!("🎯 Matched '{}' to '{}' ({}) - score: {}", 
                query, best_app?.name, best_app?.id, best_score);
            best_app
        } else {
            debug!("❌ No good match for '{}' (best score: {})", query, best_score);
            None
        }
    }
}

/// Strip field codes from Exec line
/// Field codes: %u, %U, %f, %F, %i, %c, %k
fn strip_field_codes(exec: &str) -> String {
    exec.replace("%u", "")
        .replace("%U", "")
        .replace("%f", "")
        .replace("%F", "")
        .replace("%i", "")
        .replace("%c", "")
        .replace("%k", "")
        .trim()
        .to_string()
}

/// Launch using `gio launch` (best for GNOME)
pub async fn launch_with_gio(desktop_id: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let id = desktop_id.to_string();
    let result = task::spawn_blocking(move || {
        Command::new("gio")
            .args(&["launch", &id])
            .output()
    }).await??;
    
    Ok(result.status.success())
}

/// Launch using `gtk-launch` (GTK fallback)
pub async fn launch_with_gtk(desktop_id: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    // Strip .desktop suffix for gtk-launch
    let id = desktop_id.strip_suffix(".desktop").unwrap_or(desktop_id).to_string();
    
    let result = task::spawn_blocking(move || {
        Command::new("gtk-launch")
            .arg(&id)
            .output()
    }).await??;
    
    Ok(result.status.success())
}

/// Launch using `xdg-open` (cross-desktop fallback)
pub async fn launch_with_xdg(desktop_path: &Path) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let path_str = desktop_path.to_string_lossy().to_string();
    
    let result = task::spawn_blocking(move || {
        Command::new("xdg-open")
            .arg(&path_str)
            .output()
    }).await??;
    
    Ok(result.status.success())
}

/// Launch by parsing Exec directly (last resort)
pub async fn launch_direct_exec(exec_line: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let cleaned = strip_field_codes(exec_line);
    
    let result = task::spawn_blocking(move || {
        Command::new("sh")
            .arg("-c")
            .arg(format!("nohup {} >/dev/null 2>&1 &", cleaned))
            .spawn()
    }).await??;
    
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_strip_field_codes() {
        assert_eq!(strip_field_codes("firefox %u"), "firefox");
        assert_eq!(strip_field_codes("libreoffice --writer %U"), "libreoffice --writer");
        assert_eq!(strip_field_codes("app %f %F %i %c %k"), "app");
        assert_eq!(strip_field_codes("simple-app"), "simple-app");
    }
    
    #[test]
    fn test_parse_desktop_file() {
        // Test would require a sample .desktop file
        // In production, this would test against known files
    }
}

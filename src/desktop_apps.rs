// Universal Application Launcher
// Follows FreeDesktop.org Desktop Entry Specification (Linux)
// and Info.plist parsing (macOS)
// https://specifications.freedesktop.org/desktop-entry-spec/latest/

use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::fs;
use std::process::Command;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use tokio::task;
use tracing::{info, warn, debug, error};

#[cfg(target_os = "macos")]
use plist;
#[cfg(target_os = "macos")]
use dirs;

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

impl DesktopApp {
    /// Extract binary name from Exec line
    /// 
    /// Strips field codes and extracts just the executable name
    /// without path or arguments.
    /// 
    /// # Examples
    /// 
    /// ```
    /// // "firefox %u" → "firefox"
    /// // "libreoffice --writer %U" → "libreoffice"
    /// // "/usr/bin/gnome-calculator" → "gnome-calculator"
    /// ```
    pub fn get_binary_name(&self) -> String {
        let cleaned = strip_field_codes(&self.exec);
        
        // Get first token (the binary)
        let binary_with_path = cleaned
            .split_whitespace()
            .next()
            .unwrap_or(&cleaned);
        
        // Remove path if present
        binary_with_path
            .split('/')
            .last()
            .unwrap_or(binary_with_path)
            .to_string()
    }
}

/// Launch using `gio launch` (best for GNOME)
pub async fn launch_with_gio(desktop_id: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("🔧 [ROUND6-GIO] Entering launch_with_gio() with id='{}'", desktop_id);
    let id = desktop_id.to_string();
    tracing::info!("🔧 [ROUND6-GIO] Spawning blocking task to run: gio launch {}", id);
    
    // Use spawn() instead of output() - don't wait for app to complete!
    let result = task::spawn_blocking(move || {
        tracing::info!("🔧 [ROUND6-GIO] Inside blocking task, spawning command (non-blocking)...");
        Command::new("gio")
            .args(&["launch", &id])
            .spawn()  // spawn() returns immediately, output() waits!
    }).await?;
    
    match result {
        Ok(_child) => {
            tracing::info!("✅ [ROUND6-GIO] SUCCESS! Command spawned successfully");
            Ok(true)
        }
        Err(e) => {
            tracing::info!("❌ [ROUND6-GIO] FAILED! Spawn error: {:?}", e);
            Ok(false)
        }
    }
}

/// Launch using `gtk-launch` (GTK fallback)
pub async fn launch_with_gtk(desktop_id: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("🔧 [ROUND6-GTK] Entering launch_with_gtk() with id='{}'", desktop_id);
    // Strip .desktop suffix for gtk-launch
    let id = desktop_id.strip_suffix(".desktop").unwrap_or(desktop_id).to_string();
    tracing::info!("🔧 [ROUND6-GTK] Stripped id: '{}'", id);
    tracing::info!("🔧 [ROUND6-GTK] Spawning blocking task to run: gtk-launch {}", id);
    
    // Use spawn() instead of output() - don't wait for app to complete!
    let result = task::spawn_blocking(move || {
        tracing::info!("🔧 [ROUND6-GTK] Inside blocking task, spawning command (non-blocking)...");
        Command::new("gtk-launch")
            .arg(&id)
            .spawn()  // spawn() returns immediately, output() waits 30s!
    }).await?;
    
    match result {
        Ok(_child) => {
            tracing::info!("✅ [ROUND6-GTK] SUCCESS! Command spawned successfully");
            Ok(true)
        }
        Err(e) => {
            tracing::info!("❌ [ROUND6-GTK] FAILED! Spawn error: {:?}", e);
            Ok(false)
        }
    }
}

/// Launch using `xdg-open` (cross-desktop fallback)
pub async fn launch_with_xdg(desktop_path: &Path) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("🔧 [ROUND6-XDG] Entering launch_with_xdg() with path='{}'", desktop_path.display());
    let path_str = desktop_path.to_string_lossy().to_string();
    tracing::info!("🔧 [ROUND6-XDG] Spawning blocking task to run: xdg-open {}", path_str);
    
    // Use spawn() instead of output() - don't wait for app to complete!
    let result = task::spawn_blocking(move || {
        tracing::info!("🔧 [ROUND6-XDG] Inside blocking task, spawning command (non-blocking)...");
        Command::new("xdg-open")
            .arg(&path_str)
            .spawn()  // spawn() returns immediately, output() waits!
    }).await?;
    
    match result {
        Ok(_child) => {
            tracing::info!("✅ [ROUND6-XDG] SUCCESS! Command spawned successfully");
            Ok(true)
        }
        Err(e) => {
            tracing::info!("❌ [ROUND6-XDG] FAILED! Spawn error: {:?}", e);
            Ok(false)
        }
    }
}

/// Launch by parsing Exec directly (last resort)
pub async fn launch_direct_exec(exec_line: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("🔧 [ROUND3-EXEC] Entering launch_direct_exec() with exec='{}'", exec_line);
    let cleaned = strip_field_codes(exec_line);
    tracing::info!("🔧 [ROUND3-EXEC] Cleaned exec line: '{}'", cleaned);
    let shell_cmd = format!("nohup {} >/dev/null 2>&1 &", cleaned);
    tracing::info!("🔧 [ROUND3-EXEC] Will execute: sh -c '{}'", shell_cmd);
    tracing::info!("🔧 [ROUND3-EXEC] Spawning blocking task...");
    
    match task::spawn_blocking(move || {
        tracing::info!("🔧 [ROUND3-EXEC] Inside blocking task, spawning process...");
        Command::new("sh")
            .arg("-c")
            .arg(&shell_cmd)
            .spawn()
    }).await? {
        Ok(_child) => {
            tracing::info!("✅ [ROUND3-EXEC] SUCCESS! Process spawned, returning Ok(true)");
            Ok(true)
        }
        Err(e) => {
            tracing::info!("❌ [ROUND3-EXEC] FAILED! Spawn error: {}", e);
            tracing::info!("❌ [ROUND3-EXEC] Returning Ok(false)");
            Ok(false)
        }
    }
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
    fn test_get_binary_name() {
        let app1 = DesktopApp {
            id: "firefox.desktop".to_string(),
            name: "Firefox".to_string(),
            exec: "firefox %u".to_string(),
            keywords: vec![],
            generic_name: "Web Browser".to_string(),
            path: PathBuf::from("/usr/share/applications/firefox.desktop"),
        };
        assert_eq!(app1.get_binary_name(), "firefox");
        
        let app2 = DesktopApp {
            id: "libreoffice-writer.desktop".to_string(),
            name: "LibreOffice Writer".to_string(),
            exec: "libreoffice --writer %U".to_string(),
            keywords: vec![],
            generic_name: "Word Processor".to_string(),
            path: PathBuf::from("/usr/share/applications/libreoffice-writer.desktop"),
        };
        assert_eq!(app2.get_binary_name(), "libreoffice");
        
        let app3 = DesktopApp {
            id: "calculator.desktop".to_string(),
            name: "Calculator".to_string(),
            exec: "/usr/bin/gnome-calculator".to_string(),
            keywords: vec![],
            generic_name: "Calculator".to_string(),
            path: PathBuf::from("/usr/share/applications/calculator.desktop"),
        };
        assert_eq!(app3.get_binary_name(), "gnome-calculator");
    }
    
    #[test]
    fn test_parse_desktop_file() {
        // Test would require a sample .desktop file
        // In production, this would test against known files
    }
}

// ============================================================================
// macOS Universal App Discovery
// ============================================================================

/// Represents a macOS application bundle
#[cfg(target_os = "macos")]
#[derive(Debug, Clone)]
pub struct MacApp {
    /// Application name (e.g., "Firefox.app")
    pub name: String,
    /// Bundle identifier from Info.plist (e.g., "org.mozilla.firefox")
    pub bundle_id: String,
    /// Display name (e.g., "Firefox")
    pub display_name: String,
    /// Full path to .app bundle
    pub path: PathBuf,
}

#[cfg(target_os = "macos")]
impl MacApp {
    /// Get executable name from bundle
    /// 
    /// Extracts the binary name from the .app bundle name.
    /// 
    /// # Examples
    /// 
    /// ```
    /// // "Firefox.app" → "Firefox"
    /// // "Google Chrome.app" → "Google Chrome"
    /// ```
    pub fn get_executable_name(&self) -> String {
        self.name
            .strip_suffix(".app")
            .unwrap_or(&self.name)
            .to_string()
    }
}

/// Index of all discoverable macOS applications
#[cfg(target_os = "macos")]
pub struct MacAppIndex {
    apps: Vec<MacApp>,
}

#[cfg(target_os = "macos")]
impl MacAppIndex {
    /// Scan macOS application directories and build index
    /// 
    /// Scans the following directories:
    /// - /Applications
    /// - /System/Applications
    /// - ~/Applications
    /// 
    /// Parses Info.plist files to extract bundle identifiers and display names.
    /// 
    /// # Returns
    /// 
    /// A MacAppIndex containing all discovered applications, or an error if scanning fails.
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut apps = Vec::new();
        
        // Directories to scan for .app bundles
        let search_dirs = vec![
            PathBuf::from("/Applications"),
            PathBuf::from("/System/Applications"),
            dirs::home_dir().map(|h| h.join("Applications")).unwrap_or_default(),
        ];
        
        for dir in search_dirs {
            if !dir.exists() {
                continue;
            }
            
            // Scan for .app bundles
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    
                    // Only process .app bundles
                    if path.extension().and_then(|s| s.to_str()) != Some("app") {
                        continue;
                    }
                    
                    // Parse Info.plist
                    let info_plist = path.join("Contents").join("Info.plist");
                    if let Ok(plist_data) = std::fs::read(&info_plist) {
                        if let Ok(plist) = plist::Value::from_reader(std::io::Cursor::new(plist_data)) {
                            if let Some(dict) = plist.as_dictionary() {
                                // Extract bundle ID and display name
                                let bundle_id = dict
                                    .get("CFBundleIdentifier")
                                    .and_then(|v| v.as_string())
                                    .unwrap_or("")
                                    .to_string();
                                
                                let display_name = dict
                                    .get("CFBundleName")
                                    .or_else(|| dict.get("CFBundleDisplayName"))
                                    .and_then(|v| v.as_string())
                                    .unwrap_or_else(|| {
                                        // Fallback: use filename without .app
                                        path.file_stem()
                                            .and_then(|s| s.to_str())
                                            .unwrap_or("")
                                    })
                                    .to_string();
                                
                                let name = path
                                    .file_name()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("")
                                    .to_string();
                                
                                if !bundle_id.is_empty() {
                                    apps.push(MacApp {
                                        name,
                                        bundle_id,
                                        display_name,
                                        path: path.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(MacAppIndex { apps })
    }
    
    /// Find application by name using fuzzy matching
    /// 
    /// Searches by:
    /// 1. Exact match on display name
    /// 2. Exact match on bundle name
    /// 3. Fuzzy match on display name
    /// 4. Fuzzy match on bundle name
    /// 5. Bundle ID match
    /// 
    /// # Arguments
    /// 
    /// * `query` - Application name to search for
    /// 
    /// # Returns
    /// 
    /// The best matching MacApp, or None if no match found.
    pub fn find(&self, query: &str) -> Option<&MacApp> {
        use fuzzy_matcher::FuzzyMatcher;
        use fuzzy_matcher::skim::SkimMatcherV2;
        
        let matcher = SkimMatcherV2::default();
        let query_lower = query.to_lowercase();
        
        // Try exact matches first
        for app in &self.apps {
            if app.display_name.to_lowercase() == query_lower {
                return Some(app);
            }
            if app.name.to_lowercase() == query_lower {
                return Some(app);
            }
        }
        
        // Try fuzzy matching
        let mut best_match: Option<(&MacApp, i64)> = None;
        
        for app in &self.apps {
            // Try display name
            if let Some(score) = matcher.fuzzy_match(&app.display_name.to_lowercase(), &query_lower) {
                if best_match.is_none() || score > best_match.unwrap().1 {
                    best_match = Some((app, score));
                }
            }
            
            // Try bundle name
            if let Some(score) = matcher.fuzzy_match(&app.name.to_lowercase(), &query_lower) {
                if best_match.is_none() || score > best_match.unwrap().1 {
                    best_match = Some((app, score));
                }
            }
            
            // Try bundle ID
            if let Some(score) = matcher.fuzzy_match(&app.bundle_id.to_lowercase(), &query_lower) {
                if best_match.is_none() || score > best_match.unwrap().1 {
                    best_match = Some((app, score));
                }
            }
        }
        
        best_match.map(|(app, _)| app)
    }
}

//! Configuration management for the FTP client
//!
//! This module handles loading and saving server configurations from/to JSON files.
//! It also manages the application's theme and other settings.


use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::info;
use anyhow::Context;
use ratatui::style::Color;

// Import the ProtocolType from the ftp module
use crate::ftp::ProtocolType;

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// List of saved FTP server configurations
    pub servers: Vec<ServerConfig>,
    
    /// Default local directory for downloads
    pub default_local_path: Option<String>,
    
    /// UI theme configuration
    pub theme: Option<ThemeConfig>,
    
    /// Application settings
    pub settings: Option<AppSettings>,
}

/// FTP/SFTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Display name for this server
    pub name: String,
    
    /// Server hostname or IP address
    pub host: String,
    
    /// Username for authentication
    pub username: String,
    
    /// Password (optional - can be prompted at runtime, never persisted)
    #[serde(skip)]
    pub password: Option<String>,
    
    /// Port number (default: 21 for FTP/FTPS, 22 for SFTP)
    #[serde(default = "default_port")]
    pub port: u16,
    
    /// Protocol type (FTP, FTPS, or SFTP)
    #[serde(default)]
    pub protocol: ProtocolType,
    
    /// Use secure connection (FTPS) - kept for backward compatibility
    #[serde(default)]
    pub use_tls: bool,
    
    /// Hostname for TLS verification (optional)
    #[serde(default)]
    pub tls_hostname: Option<String>,
    
    /// Passive mode (default: true)
    #[serde(default = "default_passive_mode")]
    pub passive_mode: bool,
    
    /// Connection timeout in seconds (default: 30)
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

/// UI theme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    /// Primary color for UI elements
    pub primary_color: Option<String>,
    
    /// Accent color for highlights
    pub accent_color: Option<String>,
    
    /// Background color
    pub background_color: Option<String>,
    
    /// Text color
    pub text_color: Option<String>,
    
    /// Success color (green)
    pub success_color: Option<String>,
    
    /// Error color (red)
    pub error_color: Option<String>,
    
    /// Warning color (yellow)
    pub warning_color: Option<String>,
    
    /// Info color (blue)
    pub info_color: Option<String>,
}

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Auto-save last used server
    pub auto_save_last_server: Option<bool>,
    
    /// Default transfer mode (binary/ascii/auto)
    pub default_transfer_mode: Option<String>,
    
    /// Show hidden files
    pub show_hidden_files: Option<bool>,
    
    /// Confirm before overwriting files
    pub confirm_overwrite: Option<bool>,
    
    /// Maximum concurrent transfers
    pub max_concurrent_transfers: Option<usize>,
    
    /// Transfer buffer size in bytes
    pub transfer_buffer_size: Option<usize>,
    
    /// Keep connection alive interval in seconds
    pub keep_alive_interval: Option<u64>,
    
    /// Retry attempts for failed operations
    pub retry_attempts: Option<u32>,
    
    /// Retry delay in seconds
    pub retry_delay: Option<u64>,
}

impl Config {
    /// Create a new default configuration
    pub fn new() -> Self {
        Self {
            servers: Vec::new(),
            default_local_path: Some(get_default_download_path()),
            theme: Some(ThemeConfig::default()),
            settings: Some(AppSettings::default()),
        }
    }

    /// Load configuration from file
    pub fn load_from_file(path: &str) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)
            .context("Failed to read config file")?;
        
        let config: Config = serde_json::from_str(&content)
            .context("Failed to parse config file")?;
        
        info!("Configuration loaded from: {}", path);
        Ok(config)
    }

    /// Save configuration to file
    pub fn save_to_file(&self, path: &str) -> anyhow::Result<()> {
        if let Some(parent) = PathBuf::from(path).parent() {
            fs::create_dir_all(parent)
                .context("Failed to create config directory")?;
        }

        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize config")?;
        
        fs::write(path, content)
            .context("Failed to write config file")?;
        
        info!("Configuration saved to: {}", path);
        Ok(())
    }

    /// Get default configuration path
    pub fn get_default_config_path() -> PathBuf {
        if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home)
                .join(".config")
                .join("phantomftp")
                .join("config.json")
        } else {
            PathBuf::from("config.json")
        }
    }

    /// Add a new server configuration
    pub fn add_server(&mut self, server: ServerConfig) {
        self.servers.push(server);
    }

    /// Remove a server configuration by index
    pub fn remove_server(&mut self, index: usize) -> Option<ServerConfig> {
        if index < self.servers.len() {
            Some(self.servers.remove(index))
        } else {
            None
        }
    }

    /// Get server by name
    pub fn get_server_by_name(&self, name: &str) -> Option<&ServerConfig> {
        self.servers.iter().find(|s| s.name == name)
    }

    /// Get default local path
    pub fn get_default_local_path(&self) -> String {
        self.default_local_path.clone()
            .unwrap_or_else(get_default_download_path)
    }
}

/// Default port value (21)
fn default_port() -> u16 {
    21
}

/// Default passive mode setting (true)
fn default_passive_mode() -> bool {
    true
}

/// Default timeout value (30 seconds)
fn default_timeout() -> u64 {
    30
}

impl ServerConfig {
    /// Create a new server configuration
    pub fn new(name: String, host: String, username: String) -> Self {
        Self {
            name,
            host,
            username,
            password: None,
            port: default_port(),
            protocol: ProtocolType::Ftp,
            use_tls: false,
            tls_hostname: None,
            passive_mode: default_passive_mode(),
            timeout: default_timeout(),
        }
    }

    /// Create a new secure server configuration with FTPS
    pub fn new_secure(name: String, host: String, username: String, tls_hostname: Option<String>) -> Self {
        Self {
            name,
            host,
            username,
            password: None,
            port: default_port(),
            protocol: ProtocolType::Ftps,
            use_tls: true,
            tls_hostname,
            passive_mode: default_passive_mode(),
            timeout: default_timeout(),
        }
    }

    /// Get server address (host:port)
    pub fn address(&self) -> String {
        if self.host.contains(':') {
            self.host.clone()
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }

    /// Check if password is required
    pub fn has_password(&self) -> bool {
        self.password.is_some()
    }

    /// Get display name with security indicator
    pub fn display_name(&self) -> String {
        match &self.protocol {
            ProtocolType::Ftp => self.name.clone(),
            ProtocolType::Ftps => format!("{} (Secure)", self.name),
        }
    }
}

impl ThemeConfig {
    /// Apply theme to terminal colors
    pub fn apply_theme(&self) -> HashMap<String, Color> {
        let mut colors = HashMap::new();
        
        colors.insert("primary".to_string(), parse_color(&self.primary_color));
        colors.insert("accent".to_string(), parse_color(&self.accent_color));
        colors.insert("background".to_string(), parse_color(&self.background_color));
        colors.insert("text".to_string(), parse_color(&self.text_color));
        colors.insert("success".to_string(), parse_color(&self.success_color));
        colors.insert("error".to_string(), parse_color(&self.error_color));
        colors.insert("warning".to_string(), parse_color(&self.warning_color));
        colors.insert("info".to_string(), parse_color(&self.info_color));
        
        colors
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self::new("New Server".to_string(), "localhost".to_string(), "anonymous".to_string())
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            primary_color: Some("blue".to_string()),
            accent_color: Some("yellow".to_string()),
            background_color: Some("black".to_string()),
            text_color: Some("white".to_string()),
            success_color: Some("green".to_string()),
            error_color: Some("red".to_string()),
            warning_color: Some("yellow".to_string()),
            info_color: Some("cyan".to_string()),
        }
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            auto_save_last_server: Some(true),
            default_transfer_mode: Some("binary".to_string()),
            show_hidden_files: Some(false),
            confirm_overwrite: Some(true),
            max_concurrent_transfers: Some(3),
            transfer_buffer_size: Some(8192),
            keep_alive_interval: Some(60),
            retry_attempts: Some(3),
            retry_delay: Some(2),
        }
    }
}

/// Get default download path based on OS
fn get_default_download_path() -> String {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home)
            .join("Downloads")
            .to_string_lossy()
            .to_string()
    } else {
        "./downloads".to_string()
    }
}

/// Parse color string to ratatui Color
fn parse_color(color_str: &Option<String>) -> Color {
    match color_str.as_deref() {
        Some("black") => Color::Black,
        Some("red") => Color::Red,
        Some("green") => Color::Green,
        Some("yellow") => Color::Yellow,
        Some("blue") => Color::Blue,
        Some("magenta") => Color::Magenta,
        Some("cyan") => Color::Cyan,
        Some("gray") => Color::Gray,
        Some("dark_gray") => Color::DarkGray,
        Some("light_red") => Color::LightRed,
        Some("light_green") => Color::LightGreen,
        Some("light_yellow") => Color::LightYellow,
        Some("light_blue") => Color::LightBlue,
        Some("light_magenta") => Color::LightMagenta,
        Some("light_cyan") => Color::LightCyan,
        Some("white") => Color::White,
        _ => Color::White,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = Config::new();
        assert!(config.servers.is_empty());
        assert!(config.default_local_path.is_some());
    }

    #[test]
    fn test_server_config() {
        let server = ServerConfig::new(
            "Test".to_string(),
            "example.com".to_string(),
            "user".to_string()
        );
        // Default port is 21, address() only appends if no colon present
        assert_eq!(server.address(), "example.com:21");

        let server_with_port = ServerConfig {
            port: 2121,
            ..server
        };
        assert_eq!(server_with_port.address(), "example.com:2121");
    }

    #[test]
    fn test_theme_colors() {
        let theme = ThemeConfig::default();
        let colors = theme.apply_theme();
        assert_eq!(colors.len(), 8);
    }
}
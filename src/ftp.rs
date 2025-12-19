//! FTP Manager - Handles FTP connections and file operations
//! 
//! This module provides an async wrapper around the suppaftp crate.

use anyhow::{Context, Result};
use suppaftp::tokio::AsyncFtpStream;
use suppaftp::types::FileType;
use thiserror::Error;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{debug, info, warn};
use tokio::io::AsyncReadExt;

#[derive(Debug, Error)]
pub enum FtpManagerError {
    #[error("FTP connection error: {0}")]
    ConnectionError(String),
    
    #[error("Not connected to server")]
    NotConnected,
    
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    
    #[error("Operation failed: {0}")]
    OperationFailed(String),
    
    #[error("Timeout occurred")]
    Timeout,
}

#[derive(Debug, Clone)]
pub struct RemoteFile {
    pub name: String,
    pub path: String,
    pub size: Option<u64>,
    pub is_dir: bool,
    pub modified: Option<String>,
    pub permissions: Option<String>,
}

pub struct FtpManager {
    stream: Option<AsyncFtpStream>,
    current_path: String,
    server: Option<String>,
    username: Option<String>,
}

impl FtpManager {
    pub fn new() -> Self {
        Self {
            stream: None,
            current_path: "/".to_string(),
            server: None,
            username: None,
        }
    }

    /// Connect to FTP server
    pub async fn connect(&mut self, server: &str) -> Result<()> {
        info!("Connecting to FTP server: {}", server);
        
        // Parse server address
        let server_addr = if server.contains(':') {
            server.to_string()
        } else {
            format!("{}:21", server)
        };

        // Connect with timeout
        let stream = tokio::time::timeout(
            Duration::from_secs(10),
            AsyncFtpStream::connect(&server_addr)
        )
        .await
        .context("Connection timeout")?
        .map_err(|e| FtpManagerError::ConnectionError(e.to_string()))?;
        
        self.stream = Some(stream);
        self.server = Some(server_addr.clone());
        info!("Successfully connected to {}", server_addr);
        
        Ok(())
    }

    /// Login to FTP server
    pub async fn login(&mut self, username: &str, password: &str) -> Result<()> {
        let stream = self.stream.as_mut()
            .ok_or(FtpManagerError::NotConnected)?;
        
        info!("Logging in as {}", username);
        
        stream.login(username, password)
            .await
            .map_err(|e| FtpManagerError::OperationFailed(format!("Login failed: {}", e)))?;
        
        self.username = Some(username.to_string());
        info!("Successfully logged in as {}", username);
        
        // Get current working directory after login
        self.current_path = self.pwd().await.unwrap_or_else(|_| "/".to_string());
        
        Ok(())
    }

    /// Disconnect from FTP server
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(mut stream) = self.stream.take() {
            info!("Disconnecting from FTP server");
            
            // Send QUIT command with timeout
            let result = tokio::time::timeout(
                Duration::from_secs(5),
                stream.quit()
            )
            .await;
            
            match result {
                Ok(Ok(_)) => info!("Successfully disconnected"),
                Ok(Err(e)) => warn!("Error during disconnect: {:?}", e),
                Err(_) => warn!("Disconnect timeout"),
            }
            
            self.stream = None;
            self.server = None;
            self.username = None;
        }
        Ok(())
    }

    /// List files in current directory
    pub async fn list_files(&mut self) -> Result<Vec<RemoteFile>> {
        let stream = self.stream.as_mut()
            .ok_or(FtpManagerError::NotConnected)?;
        
        debug!("Listing files in: {}", self.current_path);
        
        // List files with timeout
        let files = tokio::time::timeout(
            Duration::from_secs(30),
            stream.list(Some(&self.current_path))
        )
        .await
        .context("List operation timeout")?
        .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        
        let mut remote_files = Vec::new();
        
        for file_str in files {
            if let Ok(file) = self.parse_ftp_listing(&file_str) {
                remote_files.push(file);
            }
        }
        
        // Sort: directories first, then files, both alphabetically
        remote_files.sort_by(|a, b| {
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });
        
        Ok(remote_files)
    }

    /// Change working directory
    pub async fn change_dir(&mut self, path: &str) -> Result<()> {
        let stream = self.stream.as_mut()
            .ok_or(FtpManagerError::NotConnected)?;
        
        info!("Changing directory to: {}", path);
        
        // Handle special cases
        let target_path = if path == ".." {
            if self.current_path == "/" {
                "/".to_string()
            } else {
                let mut parts: Vec<&str> = self.current_path.split('/').collect();
                parts.pop();
                if parts.len() == 1 && parts[0].is_empty() {
                    "/".to_string()
                } else {
                    parts.join("/")
                }
            }
        } else if path.starts_with('/') {
            path.to_string()
        } else {
            if self.current_path.ends_with('/') {
                format!("{}{}", self.current_path, path)
            } else {
                format!("{}/{}", self.current_path, path)
            }
        };

        tokio::time::timeout(
            Duration::from_secs(10),
            stream.cwd(&target_path)
        )
        .await
        .context("Change directory timeout")?
        .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        
        self.current_path = target_path;
        info!("Changed directory to: {}", self.current_path);
        
        Ok(())
    }

    /// Go to parent directory
    pub async fn go_up(&mut self) -> Result<()> {
        if self.current_path != "/" {
            self.change_dir("..").await?;
        }
        Ok(())
    }

    /// Get current working directory
    pub async fn pwd(&mut self) -> Result<String> {
        let stream = self.stream.as_mut()
            .ok_or(FtpManagerError::NotConnected)?;
        
        let path = tokio::time::timeout(
            Duration::from_secs(5),
            stream.pwd()
        )
        .await
        .context("PWD timeout")?
        .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        
        Ok(path)
    }

    /// Download a file
    pub async fn download_file(&mut self, remote_path: &str, local_path: &str) -> Result<()> {
        let stream = self.stream.as_mut()
            .ok_or(FtpManagerError::NotConnected)?;
        
        info!("Downloading: {} -> {}", remote_path, local_path);
        
        // Set binary transfer mode
        stream.transfer_type(FileType::Binary)
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        
        // Create local directory if needed
        if let Some(parent) = PathBuf::from(local_path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        // Download using retr_as_stream
        let mut data_stream = stream.retr_as_stream(remote_path)
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        
        // Read all data
        let mut buf = Vec::new();
        data_stream.read_to_end(&mut buf).await?;
        
        // Finalize the stream
        stream.finalize_retr_stream(data_stream)
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        
        // Write to local file
        tokio::fs::write(local_path, buf).await?;
        
        info!("Download completed: {}", remote_path);
        Ok(())
    }

    /// Upload a file
    pub async fn upload_file(&mut self, local_path: &str, remote_path: &str) -> Result<()> {
        let stream = self.stream.as_mut()
            .ok_or(FtpManagerError::NotConnected)?;
        
        info!("Uploading: {} -> {}", local_path, remote_path);
        
        // Set binary transfer mode
        stream.transfer_type(FileType::Binary)
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        
        // Read local file
        let data = tokio::fs::read(local_path).await?;
        let mut cursor = std::io::Cursor::new(data);
        
        // Upload with timeout
        tokio::time::timeout(
            Duration::from_secs(300),
            stream.put_file(remote_path, &mut cursor)
        )
        .await
        .context("Upload timeout")?
        .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        
        info!("Upload completed: {}", remote_path);
        Ok(())
    }

    /// Create a directory
    pub async fn mkdir(&mut self, path: &str) -> Result<()> {
        let stream = self.stream.as_mut()
            .ok_or(FtpManagerError::NotConnected)?;
        
        info!("Creating directory: {}", path);
        
        tokio::time::timeout(
            Duration::from_secs(10),
            stream.mkdir(path)
        )
        .await
        .context("MKDIR timeout")?
        .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        
        Ok(())
    }

    /// Remove a file
    pub async fn remove(&mut self, path: &str) -> Result<()> {
        let stream = self.stream.as_mut()
            .ok_or(FtpManagerError::NotConnected)?;
        
        info!("Removing: {}", path);
        
        tokio::time::timeout(
            Duration::from_secs(10),
            stream.rm(path)
        )
        .await
        .context("Remove timeout")?
        .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        
        Ok(())
    }

    /// Rename a file or directory
    pub async fn rename(&mut self, from: &str, to: &str) -> Result<()> {
        let stream = self.stream.as_mut()
            .ok_or(FtpManagerError::NotConnected)?;
        
        info!("Renaming: {} -> {}", from, to);
        
        tokio::time::timeout(
            Duration::from_secs(10),
            stream.rename(from, to)
        )
        .await
        .context("Rename timeout")?
        .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        
        Ok(())
    }

    /// Get server information
    pub async fn server_info(&mut self) -> Result<String> {
        Ok("FTP Server".to_string())
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    /// Get current path
    pub fn current_path(&self) -> String {
        self.current_path.clone()
    }

    /// Get server info string
    pub fn server_info_string(&self) -> String {
        match (&self.server, &self.username) {
            (Some(server), Some(username)) => format!("{}@{}", username, server),
            (Some(server), None) => server.clone(),
            _ => "Not connected".to_string(),
        }
    }

    /// Parse FTP listing string into RemoteFile
    fn parse_ftp_listing(&self, listing: &str) -> Result<RemoteFile> {
        if listing.trim().is_empty() {
            return Err(FtpManagerError::InvalidPath("Empty listing".to_string()).into());
        }

        // Try to parse Unix-style listing first
        if let Some(file) = self.parse_unix_listing(listing) {
            return Ok(file);
        }

        // Try to parse DOS-style listing
        if let Some(file) = self.parse_dos_listing(listing) {
            return Ok(file);
        }

        // Fallback: treat as simple filename
        let name = listing.trim().to_string();
        let path = if self.current_path.ends_with('/') {
            format!("{}{}", self.current_path, name)
        } else {
            format!("{}/{}", self.current_path, name)
        };

        Ok(RemoteFile {
            name,
            path,
            size: None,
            is_dir: false,
            modified: None,
            permissions: None,
        })
    }

    fn parse_unix_listing(&self, listing: &str) -> Option<RemoteFile> {
        let parts: Vec<&str> = listing.split_whitespace().collect();
        if parts.len() < 9 {
            return None;
        }

        let permissions = parts[0];
        let is_dir = permissions.starts_with('d');
        let is_symlink = permissions.starts_with('l');
        
        let name = parts.last()?.to_string();
        let size = parts.get(4).and_then(|s| s.parse().ok());
        
        let modified = if parts.len() >= 8 {
            Some(format!("{} {} {}", parts[5], parts[6], parts[7]))
        } else {
            None
        };

        let path = if self.current_path.ends_with('/') {
            format!("{}{}", self.current_path, name)
        } else {
            format!("{}/{}", self.current_path, name)
        };

        let actual_name = if is_symlink && name.contains(" -> ") {
            name.split(" -> ").next()?.to_string()
        } else {
            name
        };

        Some(RemoteFile {
            name: actual_name,
            path,
            size,
            is_dir,
            modified,
            permissions: Some(permissions.to_string()),
        })
    }

    fn parse_dos_listing(&self, listing: &str) -> Option<RemoteFile> {
        let parts: Vec<&str> = listing.split_whitespace().collect();
        if parts.len() < 4 {
            return None;
        }

        let is_dir = parts.contains(&"<DIR>");
        let size = if is_dir {
            None
        } else {
            parts.get(2).and_then(|s| s.parse().ok())
        };

        let name = parts.last()?.to_string();

        let path = if self.current_path.ends_with('/') {
            format!("{}{}", self.current_path, name)
        } else {
            format!("{}/{}", self.current_path, name)
        };

        Some(RemoteFile {
            name,
            path,
            size,
            is_dir,
            modified: Some(format!("{} {}", parts[0], parts[1])),
            permissions: None,
        })
    }
}

impl Default for FtpManager {
    fn default() -> Self {
        Self::new()
    }
}
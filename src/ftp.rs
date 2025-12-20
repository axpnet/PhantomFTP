//! FTP Manager - Handles FTP/FTPS connections and file operations
//! 
//! This module provides an async wrapper around the suppaftp crate.
//! Note: SFTP support temporarily disabled - will be re-added in v1.1

use anyhow::Result;
use suppaftp::tokio::AsyncFtpStream;
use suppaftp::types::{FileType, FormatControl};
use thiserror::Error;
use std::path::PathBuf;
use tracing::{info, warn};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Custom error types for FTP operations
#[derive(Error, Debug)]
pub enum FtpManagerError {
    #[error("FTP connection error: {0}")]
    ConnectionError(String),
    
    #[error("Not connected to any server")]
    NotConnected,
    
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Authentication failed: {0}")]
    AuthFailed(String),
    
    #[error("Operation failed: {0}")]
    OperationFailed(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
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

/// Represents the progress of a file transfer
#[derive(Debug, Clone)]
pub struct TransferProgress {
    pub filename: String,
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub percentage: u32,
}

/// Protocol types supported by the client
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, Default)]
pub enum ProtocolType {
    #[default]
    Ftp,
    Ftps,
    // SFTP temporarily disabled - will be re-added in v1.1
    // Sftp,
}

/// Manages FTP connections and operations
pub struct FtpManager {
    /// The underlying FTP stream
    stream: Option<AsyncFtpStream>,
    
    /// Current working directory
    current_path: String,
    
    /// Server address
    address: Option<String>,
    
    /// Server name (for display)
    server: Option<String>,
    
    /// Username used for connection
    username: Option<String>,
    
    /// Protocol type used for connection
    protocol: ProtocolType,
}

impl FtpManager {
    /// Create a new FTP manager
    pub fn new() -> Self {
        Self {
            stream: None,
            current_path: "/".to_string(),
            address: None,
            server: None,
            username: None,
            protocol: ProtocolType::Ftp,
        }
    }

    /// Connect to FTP server
    pub async fn connect(&mut self, server: &str) -> Result<()> {
        info!("Connecting to FTP server: {}", server);
        
        // Parse server address - add default port if missing
        let address = if server.contains(':') {
            server.to_string()
        } else {
            format!("{}:21", server)
        };
        
        self.address = Some(address.clone());
        self.server = Some(server.to_string());
        self.protocol = ProtocolType::Ftp;
        
        // Connect to FTP server
        let stream = AsyncFtpStream::connect(&address)
            .await
            .map_err(|e| FtpManagerError::ConnectionFailed(e.to_string()))?;
        
        self.stream = Some(stream);
        info!("Connected to FTP server: {}", server);
        Ok(())
    }

    /// Connect securely using FTPS
    pub async fn connect_secure(&mut self, address: &str, _hostname: &str) -> Result<()> {
        info!("Connecting with FTPS to: {}", address);
        
        self.address = Some(address.to_string());
        self.protocol = ProtocolType::Ftps;
        
        // Note: FTPS support requires additional TLS configuration
        // For now, fall back to standard FTP connection
        warn!("FTPS not fully implemented, using standard FTP");
        
        let stream = AsyncFtpStream::connect(address)
            .await
            .map_err(|e| FtpManagerError::ConnectionFailed(e.to_string()))?;
        
        self.stream = Some(stream);
        Ok(())
    }

    /// Login to FTP server
    pub async fn login(&mut self, username: &str, password: &str) -> Result<()> {
        let stream = self.stream.as_mut()
            .ok_or(FtpManagerError::NotConnected)?;
        
        info!("Logging in as: {}", username);
        self.username = Some(username.to_string());
        
        stream.login(username, password)
            .await
            .map_err(|e| FtpManagerError::AuthFailed(e.to_string()))?;
        
        // Get initial path
        if let Ok(pwd) = stream.pwd().await {
            self.current_path = pwd;
        }
        
        info!("Login successful");
        Ok(())
    }

    /// Disconnect from FTP server
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(mut stream) = self.stream.take() {
            let _ = stream.quit().await;
        }
        
        self.address = None;
        self.server = None;
        self.username = None;
        self.current_path = "/".to_string();
        
        info!("Disconnected from server");
        Ok(())
    }

    /// List files in the current directory
    pub async fn list_files(&mut self) -> Result<Vec<RemoteFile>> {
        let stream = self.stream.as_mut()
            .ok_or(FtpManagerError::NotConnected)?;
        
        let list = stream.list(None)
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        
        let mut files = Vec::new();
        
        for entry in list {
            if let Ok(file) = self.parse_ftp_listing(&entry) {
                files.push(file);
            }
        }
        
        // Sort: directories first, then alphabetically
        files.sort_by(|a, b| {
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });
        
        Ok(files)
    }

    /// Change working directory
    pub async fn change_working_dir(&mut self, path: &str) -> Result<String> {
        let stream = self.stream.as_mut().ok_or(FtpManagerError::NotConnected)?;
        stream.cwd(path).await.map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        self.current_path = stream.pwd().await.map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        Ok(self.current_path.clone())
    }

    /// Go to parent directory
    pub async fn cd_up(&mut self) -> Result<String> {
        let stream = self.stream.as_mut().ok_or(FtpManagerError::NotConnected)?;
        stream.cwd("..").await.map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        self.current_path = stream.pwd().await.map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        Ok(self.current_path.clone())
    }

    /// Alias for cd_up (backwards compatibility)
    pub async fn go_up(&mut self) -> Result<()> {
        self.cd_up().await?;
        Ok(())
    }

    /// Alias for change_working_dir (backwards compatibility)
    pub async fn change_dir(&mut self, path: &str) -> Result<()> {
        self.change_working_dir(path).await?;
        Ok(())
    }

    /// Get current working directory
    pub async fn pwd(&mut self) -> Result<String> {
        let stream = self.stream.as_mut()
            .ok_or(FtpManagerError::NotConnected)?;
        
        let path = stream.pwd()
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        
        self.current_path = path.clone();
        Ok(path)
    }

    /// Download a file with progress callback
    pub async fn download_file<F>(&mut self, remote_path: &str, local_path: &str, progress_callback: F) -> Result<()>
    where
        F: Fn(TransferProgress) + Send + Sync,
    {
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
        
        // Get file size for progress tracking
        let file_info = stream.size(remote_path).await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        
        let total_bytes = file_info as u64;
        
        // Extract filename for progress reporting
        let filename = std::path::Path::new(remote_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        // Download using retr_as_stream
        let mut data_stream = stream.retr_as_stream(remote_path)
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        
        // Create local file
        let mut file = tokio::fs::File::create(local_path).await?;
        
        // Buffer for reading data
        let mut buffer = [0; 8192];
        let mut transferred_bytes = 0u64;
        
        loop {
            let bytes_read = data_stream.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            
            // Write to file
            file.write_all(&buffer[..bytes_read]).await?;
            
            // Update progress
            transferred_bytes += bytes_read as u64;
            
            // Report progress
            let percentage = if total_bytes > 0 {
                ((transferred_bytes as f64 / total_bytes as f64) * 100.0) as u32
            } else {
                0
            };
            
            let progress = TransferProgress {
                filename: filename.clone(),
                total_bytes,
                transferred_bytes,
                percentage,
            };
            
            progress_callback(progress);
        }
        
        // Finalize the stream
        stream.finalize_retr_stream(data_stream)
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        
        info!("Download completed: {}", remote_path);
        Ok(())
    }

    /// Simple download without progress reporting
    pub async fn download_file_simple(&mut self, remote_path: &str, local_path: &str) -> Result<()> {
        self.download_file(remote_path, local_path, |_| {}).await
    }

    /// Upload a file with progress callback
    pub async fn upload_file<F>(&mut self, local_path: &str, remote_path: &str, progress_callback: F) -> Result<()>
    where
        F: Fn(TransferProgress) + Send + Sync,
    {
        let stream = self.stream.as_mut()
            .ok_or(FtpManagerError::NotConnected)?;
        
        info!("Uploading: {} -> {}", local_path, remote_path);
        
        // Set binary transfer mode
        stream.transfer_type(FileType::Binary)
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        
        // Read local file
        let data = tokio::fs::read(local_path).await?;
        let total_bytes = data.len() as u64;
        
        // Extract filename
        let filename = std::path::Path::new(local_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        // Report initial progress
        progress_callback(TransferProgress {
            filename: filename.clone(),
            total_bytes,
            transferred_bytes: 0,
            percentage: 0,
        });
        
        // Upload file using put_with_stream
        let _data_len = data.len();
        let mut data_stream = stream.put_with_stream(remote_path)
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        
        data_stream.write_all(&data).await?;
        
        drop(data_stream);
        
        stream.finalize_put_stream(Box::new(tokio::io::empty()))
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        
        // Report completion
        progress_callback(TransferProgress {
            filename,
            total_bytes,
            transferred_bytes: total_bytes,
            percentage: 100,
        });
        
        info!("Upload completed: {}", remote_path);
        Ok(())
    }

    /// Simple upload without progress reporting
    pub async fn upload_file_simple(&mut self, local_path: &str, remote_path: &str) -> Result<()> {
        self.upload_file(local_path, remote_path, |_| {}).await
    }

    /// Get file content for preview
    pub async fn get_file_content(&mut self, remote_path: &str) -> Result<String> {
        let stream = self.stream.as_mut()
            .ok_or(FtpManagerError::NotConnected)?;
        
        // Set ASCII mode for text files
        stream.transfer_type(FileType::Ascii(FormatControl::Default))
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        
        let mut data_stream = stream.retr_as_stream(remote_path)
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        
        let mut content = String::new();
        let mut buffer = [0u8; 8192];
        
        loop {
            let bytes_read = data_stream.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            if let Ok(text) = String::from_utf8(buffer[..bytes_read].to_vec()) {
                content.push_str(&text);
            }
            // Limit preview size
            if content.len() > 10000 {
                content.push_str("\n...[truncated]...");
                break;
            }
        }
        
        stream.finalize_retr_stream(data_stream)
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        
        Ok(content)
    }

    /// Alias for get_file_content (backwards compatibility)
    pub async fn preview_file(&mut self, remote_path: &str) -> Result<String> {
        self.get_file_content(remote_path).await
    }

    /// Delete a file or directory
    pub async fn delete(&mut self, path: &str, is_dir: bool) -> Result<()> {
        let stream = self.stream.as_mut()
            .ok_or(FtpManagerError::NotConnected)?;
        
        if is_dir {
            stream.rmdir(path)
                .await
                .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        } else {
            stream.rm(path)
                .await
                .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        }
        
        Ok(())
    }

    /// Create a directory
    pub async fn mkdir(&mut self, path: &str) -> Result<()> {
        let stream = self.stream.as_mut()
            .ok_or(FtpManagerError::NotConnected)?;
        
        stream.mkdir(path)
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        
        Ok(())
    }

    /// Rename a file or directory
    pub async fn rename(&mut self, from: &str, to: &str) -> Result<()> {
        let stream = self.stream.as_mut()
            .ok_or(FtpManagerError::NotConnected)?;
        
        stream.rename(from, to)
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        
        Ok(())
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    /// Get current path
    pub fn current_path(&self) -> &str {
        &self.current_path
    }

    /// Get server name
    pub fn server_name(&self) -> Option<&str> {
        self.server.as_deref()
    }

    /// Get username
    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    /// Get protocol type
    pub fn protocol(&self) -> &ProtocolType {
        &self.protocol
    }

    /// Parse FTP listing string into RemoteFile
    fn parse_ftp_listing(&self, listing: &str) -> Result<RemoteFile> {
        // Try Unix format first, then DOS format
        if let Some(file) = self.parse_unix_listing(listing) {
            return Ok(file);
        }
        if let Some(file) = self.parse_dos_listing(listing) {
            return Ok(file);
        }
        
        Err(anyhow::anyhow!("Could not parse listing: {}", listing))
    }

    fn parse_unix_listing(&self, listing: &str) -> Option<RemoteFile> {
        // Unix format: drwxr-xr-x   2 user group  4096 Jan 01 12:00 filename
        let parts: Vec<&str> = listing.split_whitespace().collect();
        if parts.len() < 9 {
            return None;
        }
        
        let perms = parts[0];
        let is_dir = perms.starts_with('d');
        let size: u64 = parts[4].parse().unwrap_or(0);
        
        // Name is everything after the 8th part
        let name = parts[8..].join(" ");
        
        // Skip . and ..
        if name == "." || name == ".." {
            return None;
        }
        
        // Modified date
        let modified = format!("{} {} {}", parts[5], parts[6], parts[7]);
        
        let path = if self.current_path.ends_with('/') {
            format!("{}{}", self.current_path, name)
        } else {
            format!("{}/{}", self.current_path, name)
        };
        
        Some(RemoteFile {
            name,
            path,
            size: Some(size),
            is_dir,
            modified: Some(modified),
            permissions: Some(perms.to_string()),
        })
    }

    fn parse_dos_listing(&self, listing: &str) -> Option<RemoteFile> {
        // DOS format: 01-01-25  12:00PM       <DIR>          foldername
        // or:         01-01-25  12:00PM              1234 filename.txt
        let parts: Vec<&str> = listing.split_whitespace().collect();
        if parts.len() < 4 {
            return None;
        }
        
        let date = parts[0];
        let time = parts[1];
        let is_dir = parts[2] == "<DIR>";
        
        let (size, name_idx) = if is_dir {
            (0u64, 3)
        } else {
            (parts[2].parse().unwrap_or(0), 3)
        };
        
        if parts.len() <= name_idx {
            return None;
        }
        
        let name = parts[name_idx..].join(" ");
        
        if name == "." || name == ".." {
            return None;
        }
        
        let path = if self.current_path.ends_with('/') {
            format!("{}{}", self.current_path, name)
        } else {
            format!("{}/{}", self.current_path, name)
        };
        
        Some(RemoteFile {
            name,
            path,
            size: Some(size),
            is_dir,
            modified: Some(format!("{} {}", date, time)),
            permissions: None,
        })
    }
}

impl Default for FtpManager {
    fn default() -> Self {
        Self::new()
    }
}
//! FTP Manager - Handles FTP connections and file operations
//! 
//! This module provides an async wrapper around the suppaftp crate.

use anyhow::{Context, Result};
use suppaftp::tokio::AsyncFtpStream;
use suppaftp::types::FileType;
use suppaftp::NativeTlsConnector;
use thiserror::Error;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{debug, info, warn};
use tokio::io::AsyncReadExt;

// SFTP imports
use russh::{client::Config as SshConfig, Client};
use russh_sftp::{client::SftpSession, protocol::OpenFlags};
use std::sync::Arc;
use tokio::net::TcpStream;

/// Custom error types for FTP operations
#[derive(Error, Debug)]
pub enum FtpManagerError {
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
#[derive(Debug, Clone, PartialEq)]
pub enum ProtocolType {
    Ftp,
    Ftps,
    Sftp,
}

/// Manages FTP connections and operations
pub struct FtpManager {
    /// The underlying FTP stream (if connected via FTP/FTPS)
    stream: Option<AsyncFtpStream>,
    
    /// The underlying SFTP session (if connected via SFTP)
    sftp_session: Option<SftpSession>,
    
    /// Current working directory
    current_path: String,
    
    /// Server address
    address: Option<String>,
    
    /// Protocol type used for connection
    protocol: ProtocolType,
}

impl FtpManager {
    /// Create a new FTP manager
    pub fn new() -> Self {
        Self {
            stream: None,
            sftp_session: None,
            current_path: "/".to_string(),
            address: None,
            protocol: ProtocolType::Ftp,
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
        self.address = Some(server_addr.clone());
        self.protocol = ProtocolType::Ftp;
        info!("Successfully connected to {}", server_addr);
        
        Ok(())
    }

    /// Connect to an FTP server securely using FTPS
    pub async fn connect_secure(&mut self, address: &str, hostname: &str) -> Result<()> {
        info!("Connecting to FTPS server: {}", address);
        
        // Create a TLS connector
        let tls_connector = NativeTlsConnector::new()
            .context("Failed to create TLS connector")?;

        // Connect to the server
        let stream = AsyncFtpStream::connect(address)
            .await
            .map_err(|e| FtpManagerError::ConnectionFailed(e.to_string()))?;

        // Upgrade to secure connection
        let mut secure_stream = stream.into_secure(tls_connector, hostname)
            .await
            .map_err(|e| FtpManagerError::ConnectionFailed(e.to_string()))?;

        // Store the stream
        self.stream = Some(secure_stream);
        self.address = Some(address.to_string());
        self.protocol = ProtocolType::Ftps;
        
        info!("Connected to FTPS server: {}", address);
        Ok(())
    }

    /// Connect to an SFTP server
    pub async fn connect_sftp(&mut self, address: &str, username: &str, password: &str) -> Result<()> {
        info!("Connecting to SFTP server: {}", address);
        
        // Parse address to separate host and port
        let parts: Vec<&str> = address.split(':').collect();
        let host = parts[0];
        let port = parts.get(1).and_then(|p| p.parse::<u16>().ok()).unwrap_or(22);
        
        // Create SSH client configuration
        let config = Arc::new(SshConfig::default());
        
        // Establish TCP connection
        let tcp_stream = TcpStream::connect((host, port))
            .await
            .context("Failed to connect to SSH server")?;
        
        // Create SSH connection
        let (ssh_connection, ssh_handle) = Client::connect(config, host, tcp_stream)
            .await
            .context("Failed to establish SSH connection")?;
        
        // Authenticate with password
        let authenticated = ssh_handle.authenticate_password(username, password)
            .await
            .context("Failed to authenticate with SSH server")?;
        
        if !authenticated {
            return Err(FtpManagerError::AuthFailed("SSH authentication failed".to_string()).into());
        }
        
        // Open SFTP subsystem
        let sftp_channel = ssh_handle.channel_open_session()
            .await
            .context("Failed to open SSH session")?;
        
        sftp_channel.request_subsystem(true, "sftp")
            .await
            .context("Failed to request SFTP subsystem")?;
        
        // Create SFTP session
        let sftp_session = SftpSession::new(sftp_channel.into_stream())
            .await
            .context("Failed to initialize SFTP session")?;
        
        // Initialize SFTP session
        sftp_session.init()
            .await
            .context("Failed to initialize SFTP protocol")?;
        
        // Store the session
        self.sftp_session = Some(sftp_session);
        self.address = Some(address.to_string());
        self.protocol = ProtocolType::Sftp;
        
        // Get initial working directory
        if let Some(session) = &self.sftp_session {
            let workdir = session.working_directory()
                .await
                .context("Failed to get working directory")?;
            self.current_path = workdir;
        }
        
        info!("Connected to SFTP server: {}", address);
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

    /// List files in the current directory
    pub async fn list_files(&mut self) -> Result<Vec<RemoteFile>> {
        match self.protocol {
            ProtocolType::Ftp | ProtocolType::Ftps => {
                // Existing FTP/FTPS implementation
                let stream = self.stream.as_mut()
                    .ok_or(FtpManagerError::NotConnected)?;

                // Change to current directory
                stream.cwd(&self.current_path)
                    .await
                    .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;

                // List files
                let files = stream.list(None)
                    .await
                    .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;

                // Parse file listings
                let mut parsed_files = Vec::new();
                for file_str in files {
                    if let Ok(file) = self.parse_ftp_listing(&file_str) {
                        parsed_files.push(file);
                    }
                }

                Ok(parsed_files)
            }
            ProtocolType::Sftp => {
                // SFTP implementation
                let session = self.sftp_session.as_ref()
                    .ok_or(FtpManagerError::NotConnected)?;

                // List files in current directory
                let entries = session.readdir(&self.current_path)
                    .await
                    .map_err(|e| FtpManagerError::OperationFailed(format!("SFTP readdir failed: {:?}", e)))?;

                let mut parsed_files = Vec::new();
                for entry in entries {
                    let is_dir = entry.attrs.is_dir();
                    let size = if is_dir { None } else { Some(entry.attrs.size.unwrap_or(0)) };
                    
                    let path = if self.current_path.ends_with('/') {
                        format!("{}{}", self.current_path, entry.filename)
                    } else {
                        format!("{}/{}", self.current_path, entry.filename)
                    };
                    
                    parsed_files.push(RemoteFile {
                        name: entry.filename,
                        path,
                        size,
                        is_dir,
                        modified: None, // Would need to parse from entry.attrs.mtime
                        permissions: None, // Would need to convert from entry.attrs.permissions
                    });
                }

                Ok(parsed_files)
            }
        }
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
                // 正确处理父目录导航
                let mut parts: Vec<&str> = self.current_path.trim_start_matches('/').split('/').collect();
                if parts.last() == Some(&"") {
                    parts.pop(); // 移除最后一个空字符串（如果路径以/结尾）
                }
                if !parts.is_empty() && parts != [""] {
                    parts.pop(); // 移除最后一个目录部分
                }
                if parts.is_empty() || parts == [""] {
                    "/".to_string()
                } else {
                    format!("/{}", parts.join("/"))
                }
            }
        } else if path.starts_with('/') {
            path.to_string()
        } else {
            // 相对路径处理
            if self.current_path == "/" {
                format!("/{}", path)
            } else if self.current_path.ends_with('/') {
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
        
        self.current_path = if target_path.is_empty() { "/".to_string() } else { target_path };
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
    pub async fn download_file<F>(&mut self, remote_path: &str, local_path: &str, progress_callback: F) -> Result<()>
    where
        F: Fn(TransferProgress) + Send + Sync,
    {
        match self.protocol {
            ProtocolType::Ftp | ProtocolType::Ftps => {
                // Existing FTP/FTPS implementation
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
                
                let total_bytes = file_info.unwrap_or(0);
                
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
                let mut buffer = [0; 8192]; // 8KB buffer
                let mut transferred_bytes = 0u64;
                
                loop {
                    let bytes_read = data_stream.read(&mut buffer).await?;
                    if bytes_read == 0 {
                        break;
                    }
                    
                    // Write to file
                    tokio::io::AsyncWriteExt::write_all(&mut file, &buffer[..bytes_read]).await?;
                    
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
            ProtocolType::Sftp => {
                // SFTP implementation
                let session = self.sftp_session.as_ref()
                    .ok_or(FtpManagerError::NotConnected)?;
                
                info!("Downloading (SFTP): {} -> {}", remote_path, local_path);
                
                // Create local directory if needed
                if let Some(parent) = PathBuf::from(local_path).parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                
                // Open remote file
                let mut file = session.open(remote_path.into())
                    .await
                    .map_err(|e| FtpManagerError::OperationFailed(format!("Failed to open remote file: {:?}", e)))?;
                
                // Get file size for progress tracking
                let attrs = file.metadata()
                    .await
                    .map_err(|e| FtpManagerError::OperationFailed(format!("Failed to get file metadata: {:?}", e)))?;
                
                let total_bytes = attrs.size.unwrap_or(0);
                
                // Extract filename for progress reporting
                let filename = std::path::Path::new(remote_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                
                // Create local file
                let mut local_file = tokio::fs::File::create(local_path).await?;
                
                // Buffer for reading data
                let mut buffer = [0; 8192]; // 8KB buffer
                let mut transferred_bytes = 0u64;
                
                loop {
                    // Read from remote file
                    let bytes_read = file.read(&mut buffer)
                        .await
                        .map_err(|e| FtpManagerError::OperationFailed(format!("Failed to read from remote file: {:?}", e)))?;
                    
                    if bytes_read == 0 {
                        break;
                    }
                    
                    // Write to local file
                    tokio::io::AsyncWriteExt::write_all(&mut local_file, &buffer[..bytes_read]).await?;
                    
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
                
                info!("SFTP download completed: {}", remote_path);
                Ok(())
            }
        }
    }
    
    /// Simple download without progress reporting (for backward compatibility)
    pub async fn download_file_simple(&mut self, remote_path: &str, local_path: &str) -> Result<()> {
        self.download_file(remote_path, local_path, |_| {}).await
    }

    /// Upload a file
    pub async fn upload_file<F>(&mut self, local_path: &str, remote_path: &str, progress_callback: F) -> Result<()>
    where
        F: Fn(TransferProgress) + Send + Sync,
    {
        match self.protocol {
            ProtocolType::Ftp | ProtocolType::Ftps => {
                // Existing FTP/FTPS implementation
                let stream = self.stream.as_mut()
                    .ok_or(FtpManagerError::NotConnected)?;
                
                info!("Uploading: {} -> {}", local_path, remote_path);
                
                // Set binary transfer mode
                stream.transfer_type(FileType::Binary)
                    .await
                    .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
                
                // Get file size for progress tracking
                let metadata = tokio::fs::metadata(local_path).await?;
                let total_bytes = metadata.len();
                
                // Extract filename for progress reporting
                let filename = std::path::Path::new(local_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                
                // Open local file
                let mut file = tokio::fs::File::open(local_path).await?;
                
                // Create a custom reader that reports progress
                struct ProgressReader<R, F> {
                    reader: R,
                    total_bytes: u64,
                    transferred_bytes: u64,
                    filename: String,
                    callback: F,
                }
                
                impl<R, F> ProgressReader<R, F>
                where
                    R: tokio::io::AsyncRead + Unpin,
                    F: Fn(TransferProgress),
                {
                    fn new(reader: R, total_bytes: u64, filename: String, callback: F) -> Self {
                        Self {
                            reader,
                            total_bytes,
                            transferred_bytes: 0,
                            filename,
                            callback,
                        }
                    }
                }
                
                impl<R, F> tokio::io::AsyncRead for ProgressReader<R, F>
                where
                    R: tokio::io::AsyncRead + Unpin,
                    F: Fn(TransferProgress),
                {
                    fn poll_read(
                        mut self: std::pin::Pin<&mut Self>,
                        cx: &mut std::task::Context<'_>,
                        buf: &mut tokio::io::ReadBuf<'_>,
                    ) -> std::task::Poll<std::io::Result<()>> {
                        let before = buf.filled().len();
                        match std::pin::Pin::new(&mut self.reader).poll_read(cx, buf) {
                            std::task::Poll::Ready(Ok(())) => {
                                let after = buf.filled().len();
                                let bytes_read = after - before;
                                self.transferred_bytes += bytes_read as u64;
                                
                                // Report progress
                                let percentage = if self.total_bytes > 0 {
                                    ((self.transferred_bytes as f64 / self.total_bytes as f64) * 100.0) as u32
                                } else {
                                    0
                                };
                                
                                let progress = TransferProgress {
                                    filename: self.filename.clone(),
                                    total_bytes: self.total_bytes,
                                    transferred_bytes: self.transferred_bytes,
                                    percentage,
                                };
                                
                                (self.callback)(progress);
                                
                                std::task::Poll::Ready(Ok(()))
                            }
                            other => other,
                        }
                    }
                }
                
                let progress_reader = ProgressReader::new(
                    &mut file,
                    total_bytes,
                    filename.clone(),
                    progress_callback,
                );
                
                // Upload with timeout
                tokio::time::timeout(
                    Duration::from_secs(300),
                    stream.put_file(remote_path, &mut std::pin::Pin::new(&mut Box::new(progress_reader)))
                )
                .await
                .context("Upload timeout")?
                .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
                
                info!("Upload completed: {}", remote_path);
                Ok(())
            }
            ProtocolType::Sftp => {
                // SFTP implementation
                let session = self.sftp_session.as_ref()
                    .ok_or(FtpManagerError::NotConnected)?;
                
                info!("Uploading (SFTP): {} -> {}", local_path, remote_path);
                
                // Get file size for progress tracking
                let metadata = tokio::fs::metadata(local_path).await?;
                let total_bytes = metadata.len();
                
                // Extract filename for progress reporting
                let filename = std::path::Path::new(local_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                
                // Open remote file for writing
                let mut remote_file = session.open_with_flags(
                    remote_path.into(),
                    OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNCATE
                )
                .await
                .map_err(|e| FtpManagerError::OperationFailed(format!("Failed to open remote file: {:?}", e)))?;
                
                // Open local file
                let mut local_file = tokio::fs::File::open(local_path).await?;
                
                // Create a custom reader that reports progress
                struct SftpProgressReader<R, F> {
                    reader: R,
                    total_bytes: u64,
                    transferred_bytes: u64,
                    filename: String,
                    callback: F,
                }
                
                impl<R, F> SftpProgressReader<R, F>
                where
                    R: tokio::io::AsyncRead + Unpin,
                    F: Fn(TransferProgress),
                {
                    fn new(reader: R, total_bytes: u64, filename: String, callback: F) -> Self {
                        Self {
                            reader,
                            total_bytes,
                            transferred_bytes: 0,
                            filename,
                            callback,
                        }
                    }
                }
                
                impl<R, F> tokio::io::AsyncRead for SftpProgressReader<R, F>
                where
                    R: tokio::io::AsyncRead + Unpin,
                    F: Fn(TransferProgress),
                {
                    fn poll_read(
                        mut self: std::pin::Pin<&mut Self>,
                        cx: &mut std::task::Context<'_>,
                        buf: &mut tokio::io::ReadBuf<'_>,
                    ) -> std::task::Poll<std::io::Result<()>> {
                        let before = buf.filled().len();
                        match std::pin::Pin::new(&mut self.reader).poll_read(cx, buf) {
                            std::task::Poll::Ready(Ok(())) => {
                                let after = buf.filled().len();
                                let bytes_read = after - before;
                                self.transferred_bytes += bytes_read as u64;
                                
                                // Report progress
                                let percentage = if self.total_bytes > 0 {
                                    ((self.transferred_bytes as f64 / self.total_bytes as f64) * 100.0) as u32
                                } else {
                                    0
                                };
                                
                                let progress = TransferProgress {
                                    filename: self.filename.clone(),
                                    total_bytes: self.total_bytes,
                                    transferred_bytes: self.transferred_bytes,
                                    percentage,
                                };
                                
                                (self.callback)(progress);
                                
                                std::task::Poll::Ready(Ok(()))
                            }
                            other => other,
                        }
                    }
                }
                
                let progress_reader = SftpProgressReader::new(
                    &mut local_file,
                    total_bytes,
                    filename.clone(),
                    progress_callback,
                );
                
                // Buffer for reading data
                let mut buffer = [0; 8192]; // 8KB buffer
                
                // Upload file
                loop {
                    let bytes_read = progress_reader.reader.read(&mut buffer)
                        .await
                        .map_err(|e| FtpManagerError::OperationFailed(format!("Failed to read from local file: {:?}", e)))?;
                    
                    if bytes_read == 0 {
                        break;
                    }
                    
                    remote_file.write_all(&buffer[..bytes_read])
                        .await
                        .map_err(|e| FtpManagerError::OperationFailed(format!("Failed to write to remote file: {:?}", e)))?;
                }
                
                info!("SFTP upload completed: {}", remote_path);
                Ok(())
            }
        }
    }
    
    // Fallback method for backward compatibility
    pub async fn upload_file_simple(&mut self, local_path: &str, remote_path: &str) -> Result<()> {
        self.upload_file(local_path, remote_path, |_| {}).await
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

    /// Preview a file's content (first 1024 bytes)
    pub async fn preview_file(&mut self, remote_path: &str) -> Result<String> {
        match self.protocol {
            ProtocolType::Ftp | ProtocolType::Ftps => {
                let stream = self.stream.as_mut()
                    .ok_or(FtpManagerError::NotConnected)?;
                
                // Set ASCII transfer mode for text preview
                stream.transfer_type(FileType::Ascii)
                    .await
                    .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
                
                // Get file preview using retr_as_stream
                let mut data_stream = stream.retr_as_stream(remote_path)
                    .await
                    .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
                
                // Read first 1024 bytes
                let mut buffer = [0; 1024];
                let bytes_read = data_stream.read(&mut buffer).await?;
                
                // Finalize the stream
                stream.finalize_retr_stream(data_stream)
                    .await
                    .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
                
                // Convert to string
                let content = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                Ok(content)
            }
            ProtocolType::Sftp => {
                let session = self.sftp_session.as_ref()
                    .ok_or(FtpManagerError::NotConnected)?;
                
                // Open remote file
                let mut file = session.open(remote_path.into())
                    .await
                    .map_err(|e| FtpManagerError::OperationFailed(format!("Failed to open remote file: {:?}", e)))?;
                
                // Read first 1024 bytes
                let mut buffer = [0; 1024];
                let bytes_read = file.read(&mut buffer)
                    .await
                    .map_err(|e| FtpManagerError::OperationFailed(format!("Failed to read from remote file: {:?}", e)))?;
                
                // Convert to string
                let content = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                Ok(content)
            }
        }
    }
    
    /// Parse FTP listing string into RemoteFile
    fn parse_ftp_listing(&self, listing: &str) -> Result<RemoteFile> {
        if listing.trim().is_empty() {
            return Err(FtpManagerError::InvalidPath("Empty listing".to_string()).into());
        }

        debug!("Parsing FTP listing: {}", listing);

        // Try to parse Unix-style listing first
        if let Some(file) = self.parse_unix_listing(listing) {
            debug!("Parsed as Unix: {} (is_dir: {})", file.name, file.is_dir);
            return Ok(file);
        }

        // Try to parse DOS-style listing
        if let Some(file) = self.parse_dos_listing(listing) {
            debug!("Parsed as DOS: {} (is_dir: {})", file.name, file.is_dir);
            return Ok(file);
        }

        // Fallback: treat as simple filename
        // Check if it might be a directory (no extension, common dir indicators)
        let name = listing.trim().to_string();
        // 更准确地判断目录：以斜杠结尾或者是一些特殊目录名
        let is_likely_dir = name.ends_with('/') || name == "." || name == ".." || !name.contains('.');

        debug!("Fallback parsing: {} (guessed is_dir: {})", name, is_likely_dir);
        
        let path = if self.current_path.ends_with('/') {
            format!("{}{}", self.current_path, name)
        } else {
            format!("{}/{}", self.current_path, name)
        };

        Ok(RemoteFile {
            name: name.trim_end_matches('/').to_string(), // 移除末尾的斜杠
            path,
            size: None,
            is_dir: is_likely_dir,
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
        
        let name = parts[8].to_string(); // Unix listing中文件名通常在第9个字段
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

        // 处理符号链接
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

        let is_dir = parts.contains(&"<DIR>") || (parts.len() >= 4 && parts[2] == "<DIR>");
        let size = if is_dir {
            None
        } else {
            parts.get(2).and_then(|s| s.parse().ok())
        };

        // DOS listing通常有3个字段：日期 时间 <DIR> 或者 文件大小 文件名
        let name_index = if is_dir { 3 } else { 3 }; // 文件名通常在第4个位置
        if parts.len() <= name_index {
            return None;
        }
        
        let name = parts[name_index].to_string();

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
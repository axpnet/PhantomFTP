//! FTP Manager — Handles FTP/FTPS connections and file operations
//!
//! Provides an async wrapper around suppaftp with real TLS support,
//! streaming transfers, progress reporting, and cancellation.

use anyhow::Result;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use suppaftp::tokio::{AsyncNativeTlsConnector, AsyncNativeTlsFtpStream};
use suppaftp::types::{FileType, FormatControl};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tracing::info;

/// Transfer buffer size (8 KiB)
const BUFFER_SIZE: usize = 8192;

// ── Error Types ──────────────────────────────────────────────────

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

    #[error("Transfer cancelled")]
    Cancelled,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

// ── Data Types ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RemoteFile {
    pub name: String,
    pub path: String,
    pub size: Option<u64>,
    pub is_dir: bool,
    pub modified: Option<String>,
    pub permissions: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TransferProgress {
    pub filename: String,
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub percentage: u32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, Default)]
pub enum ProtocolType {
    #[default]
    Ftp,
    Ftps,
}

// ── FTP Manager ──────────────────────────────────────────────────

/// Uses `AsyncNativeTlsFtpStream` for both plain FTP and FTPS connections.
/// Plain connections simply skip the `into_secure()` upgrade step.
pub struct FtpManager {
    stream: Option<AsyncNativeTlsFtpStream>,
    current_path: String,
    server: Option<String>,
    username: Option<String>,
    protocol: ProtocolType,
    /// Shared cancellation flag — set to true to abort active transfers
    pub cancelled: Arc<AtomicBool>,
}

impl FtpManager {
    pub fn new() -> Self {
        Self {
            stream: None,
            current_path: "/".to_string(),
            server: None,
            username: None,
            protocol: ProtocolType::Ftp,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Signal cancellation of active transfer
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// Reset cancellation flag before starting a new transfer
    pub fn reset_cancel(&self) {
        self.cancelled.store(false, Ordering::Relaxed);
    }

    #[cfg(test)]
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    // ── Connection ───────────────────────────────────────────────

    /// Connect to FTP server (plain, no TLS)
    pub async fn connect(&mut self, server: &str) -> Result<()> {
        info!("Connecting to FTP server: {}", server);

        let address = if server.contains(':') {
            server.to_string()
        } else {
            format!("{}:21", server)
        };

        self.server = Some(server.to_string());
        self.protocol = ProtocolType::Ftp;

        let stream = AsyncNativeTlsFtpStream::connect(&address)
            .await
            .map_err(|e| FtpManagerError::ConnectionFailed(e.to_string()))?;

        self.stream = Some(stream);
        info!("Connected to FTP server: {}", server);
        Ok(())
    }

    /// Connect to FTPS server (explicit TLS upgrade)
    pub async fn connect_secure(&mut self, server: &str, hostname: &str) -> Result<()> {
        info!("Connecting with FTPS to: {}", server);

        let address = if server.contains(':') {
            server.to_string()
        } else {
            format!("{}:21", server)
        };

        self.server = Some(server.to_string());
        self.protocol = ProtocolType::Ftps;

        // Connect plain first
        let stream = AsyncNativeTlsFtpStream::connect(&address)
            .await
            .map_err(|e| FtpManagerError::ConnectionFailed(e.to_string()))?;

        // Upgrade to TLS
        let tls = suppaftp::async_native_tls::TlsConnector::new()
            .danger_accept_invalid_certs(false);
        let connector = AsyncNativeTlsConnector::from(tls);

        let secure_stream = stream
            .into_secure(connector, hostname)
            .await
            .map_err(|e| FtpManagerError::ConnectionFailed(format!("TLS upgrade failed: {}", e)))?;

        self.stream = Some(secure_stream);
        info!("FTPS connection established: {}", server);
        Ok(())
    }

    /// Login to FTP server
    pub async fn login(&mut self, username: &str, password: &str) -> Result<()> {
        self.username = Some(username.to_string());
        let stream = self.stream.as_mut().ok_or(FtpManagerError::NotConnected)?;

        info!("Logging in as: {}", username);

        stream
            .login(username, password)
            .await
            .map_err(|e| FtpManagerError::AuthFailed(e.to_string()))?;

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

        self.server = None;
        self.username = None;
        self.current_path = "/".to_string();

        info!("Disconnected from server");
        Ok(())
    }

    // ── Directory Operations ─────────────────────────────────────

    /// List files in the current directory
    pub async fn list_files(&mut self) -> Result<Vec<RemoteFile>> {
        let stream = self.stream.as_mut().ok_or(FtpManagerError::NotConnected)?;

        let list = stream
            .list(None)
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;

        let mut files = Vec::new();
        for entry in list {
            if let Ok(file) = self.parse_ftp_listing(&entry) {
                files.push(file);
            }
        }

        // Sort: directories first, then alphabetically
        files.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        Ok(files)
    }

    /// Change working directory
    pub async fn change_dir(&mut self, path: &str) -> Result<()> {
        let stream = self.stream.as_mut().ok_or(FtpManagerError::NotConnected)?;
        stream
            .cwd(path)
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        self.current_path = stream
            .pwd()
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        Ok(())
    }

    /// Go to parent directory
    pub async fn go_up(&mut self) -> Result<()> {
        self.change_dir("..").await
    }

    /// Get current working directory from server
    pub async fn pwd(&mut self) -> Result<String> {
        let stream = self.stream.as_mut().ok_or(FtpManagerError::NotConnected)?;
        let path = stream
            .pwd()
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        self.current_path = path.clone();
        Ok(path)
    }

    // ── File Transfers ───────────────────────────────────────────

    /// Download a file with progress callback and cancellation support
    pub async fn download_file<F>(
        &mut self,
        remote_path: &str,
        local_path: &str,
        progress_callback: F,
    ) -> Result<()>
    where
        F: Fn(TransferProgress) + Send + Sync,
    {
        info!("Downloading: {} -> {}", remote_path, local_path);
        self.cancelled.store(false, Ordering::Relaxed);
        let cancelled = Arc::clone(&self.cancelled);

        let stream = self.stream.as_mut().ok_or(FtpManagerError::NotConnected)?;

        stream
            .transfer_type(FileType::Binary)
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;

        // Create local directory if needed
        if let Some(parent) = PathBuf::from(local_path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Get file size for progress tracking
        let total_bytes = stream
            .size(remote_path)
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))? as u64;

        let filename = std::path::Path::new(remote_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Download using streaming
        let mut data_stream = stream
            .retr_as_stream(remote_path)
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;

        let mut file = tokio::fs::File::create(local_path).await?;
        let mut buffer = [0u8; BUFFER_SIZE];
        let mut transferred_bytes = 0u64;

        loop {
            if cancelled.load(Ordering::Relaxed) {
                drop(data_stream);
                let _ = tokio::fs::remove_file(local_path).await;
                return Err(FtpManagerError::Cancelled.into());
            }

            let bytes_read = data_stream.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }

            file.write_all(&buffer[..bytes_read]).await?;
            transferred_bytes += bytes_read as u64;

            let percentage = if total_bytes > 0 {
                ((transferred_bytes as f64 / total_bytes as f64) * 100.0) as u32
            } else {
                0
            };

            progress_callback(TransferProgress {
                filename: filename.clone(),
                total_bytes,
                transferred_bytes,
                percentage,
            });
        }

        stream
            .finalize_retr_stream(data_stream)
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;

        info!("Download completed: {}", remote_path);
        Ok(())
    }

    /// Upload a file with streaming, progress callback, and cancellation support
    pub async fn upload_file<F>(
        &mut self,
        local_path: &str,
        remote_path: &str,
        progress_callback: F,
    ) -> Result<()>
    where
        F: Fn(TransferProgress) + Send + Sync,
    {
        info!("Uploading: {} -> {}", local_path, remote_path);
        self.cancelled.store(false, Ordering::Relaxed);
        let cancelled = Arc::clone(&self.cancelled);

        let stream = self.stream.as_mut().ok_or(FtpManagerError::NotConnected)?;

        stream
            .transfer_type(FileType::Binary)
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;

        let metadata = tokio::fs::metadata(local_path).await?;
        let total_bytes = metadata.len();

        let filename = std::path::Path::new(local_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let file = tokio::fs::File::open(local_path).await?;
        let mut reader = BufReader::new(file);

        let mut data_stream = stream
            .put_with_stream(remote_path)
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;

        let mut buffer = [0u8; BUFFER_SIZE];
        let mut transferred_bytes = 0u64;

        loop {
            if cancelled.load(Ordering::Relaxed) {
                return Err(FtpManagerError::Cancelled.into());
            }

            let bytes_read = reader.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }

            data_stream.write_all(&buffer[..bytes_read]).await?;
            transferred_bytes += bytes_read as u64;

            let percentage = if total_bytes > 0 {
                ((transferred_bytes as f64 / total_bytes as f64) * 100.0) as u32
            } else {
                0
            };

            progress_callback(TransferProgress {
                filename: filename.clone(),
                total_bytes,
                transferred_bytes,
                percentage,
            });
        }

        stream
            .finalize_put_stream(data_stream)
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;

        info!("Upload completed: {}", remote_path);
        Ok(())
    }

    /// Simple download without progress reporting
    pub async fn download_file_simple(
        &mut self,
        remote_path: &str,
        local_path: &str,
    ) -> Result<()> {
        self.download_file(remote_path, local_path, |_| {}).await
    }

    /// Simple upload without progress reporting
    pub async fn upload_file_simple(
        &mut self,
        local_path: &str,
        remote_path: &str,
    ) -> Result<()> {
        self.upload_file(local_path, remote_path, |_| {}).await
    }

    // ── File Operations ──────────────────────────────────────────

    /// Get file content for preview (text files, max 10KB)
    pub async fn preview_file(&mut self, remote_path: &str) -> Result<String> {
        let stream = self.stream.as_mut().ok_or(FtpManagerError::NotConnected)?;

        stream
            .transfer_type(FileType::Ascii(FormatControl::Default))
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;

        let mut data_stream = stream
            .retr_as_stream(remote_path)
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;

        let mut content = String::new();
        let mut buffer = [0u8; BUFFER_SIZE];

        loop {
            let bytes_read = data_stream.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            if let Ok(text) = String::from_utf8(buffer[..bytes_read].to_vec()) {
                content.push_str(&text);
            }
            if content.len() > 10000 {
                content.push_str("\n...[truncated]...");
                break;
            }
        }

        stream
            .finalize_retr_stream(data_stream)
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;

        Ok(content)
    }

    /// Delete a file or directory
    pub async fn delete(&mut self, path: &str, is_dir: bool) -> Result<()> {
        let stream = self.stream.as_mut().ok_or(FtpManagerError::NotConnected)?;

        if is_dir {
            stream
                .rmdir(path)
                .await
                .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        } else {
            stream
                .rm(path)
                .await
                .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        }

        Ok(())
    }

    /// Create a directory
    pub async fn mkdir(&mut self, path: &str) -> Result<()> {
        let stream = self.stream.as_mut().ok_or(FtpManagerError::NotConnected)?;
        stream
            .mkdir(path)
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        Ok(())
    }

    /// Rename a file or directory
    pub async fn rename(&mut self, from: &str, to: &str) -> Result<()> {
        let stream = self.stream.as_mut().ok_or(FtpManagerError::NotConnected)?;
        stream
            .rename(from, to)
            .await
            .map_err(|e| FtpManagerError::OperationFailed(e.to_string()))?;
        Ok(())
    }

    // ── Getters ──────────────────────────────────────────────────

    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    pub fn current_path(&self) -> &str {
        &self.current_path
    }

    pub fn server_name(&self) -> Option<&str> {
        self.server.as_deref()
    }

    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    pub fn protocol(&self) -> &ProtocolType {
        &self.protocol
    }

    // ── FTP Listing Parsers ──────────────────────────────────────

    fn parse_ftp_listing(&self, listing: &str) -> Result<RemoteFile> {
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

        // Name is everything after the 8th column (handles spaces in names)
        let name = parts[8..].join(" ");

        if name == "." || name == ".." {
            return None;
        }

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

        let (size, name_start) = if is_dir {
            (0u64, 3)
        } else {
            let size = parts[2].parse().unwrap_or(0);
            (size, 3)
        };

        if parts.len() <= name_start {
            return None;
        }

        let name = parts[name_start..].join(" ");

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

// ── Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn manager_at(path: &str) -> FtpManager {
        let mut m = FtpManager::new();
        m.current_path = path.to_string();
        m
    }

    #[test]
    fn test_parse_unix_directory() {
        let m = manager_at("/home");
        let file = m
            .parse_unix_listing("drwxr-xr-x   2 user group  4096 Jan 01 12:00 Documents")
            .unwrap();
        assert!(file.is_dir);
        assert_eq!(file.name, "Documents");
        assert_eq!(file.path, "/home/Documents");
        assert_eq!(file.size, Some(4096));
    }

    #[test]
    fn test_parse_unix_file() {
        let m = manager_at("/var/www");
        let file = m
            .parse_unix_listing("-rw-r--r--   1 user group  12345 Mar 15 09:30 index.html")
            .unwrap();
        assert!(!file.is_dir);
        assert_eq!(file.name, "index.html");
        assert_eq!(file.path, "/var/www/index.html");
        assert_eq!(file.size, Some(12345));
    }

    #[test]
    fn test_parse_unix_filename_with_spaces() {
        let m = manager_at("/");
        let file = m
            .parse_unix_listing("-rw-r--r--   1 user group  100 Jan 01 00:00 my cool file.txt")
            .unwrap();
        assert_eq!(file.name, "my cool file.txt");
    }

    #[test]
    fn test_parse_unix_skips_dot_entries() {
        let m = manager_at("/");
        assert!(m
            .parse_unix_listing("drwxr-xr-x   2 user group  4096 Jan 01 12:00 .")
            .is_none());
        assert!(m
            .parse_unix_listing("drwxr-xr-x   2 user group  4096 Jan 01 12:00 ..")
            .is_none());
    }

    #[test]
    fn test_parse_dos_directory() {
        let m = manager_at("/");
        let file = m
            .parse_dos_listing("01-15-26  03:30PM       <DIR>          Photos")
            .unwrap();
        assert!(file.is_dir);
        assert_eq!(file.name, "Photos");
        assert_eq!(file.size, Some(0));
    }

    #[test]
    fn test_parse_dos_file() {
        let m = manager_at("/data");
        let file = m
            .parse_dos_listing("01-15-26  03:30PM              54321 report.pdf")
            .unwrap();
        assert!(!file.is_dir);
        assert_eq!(file.name, "report.pdf");
        assert_eq!(file.path, "/data/report.pdf");
        assert_eq!(file.size, Some(54321));
    }

    #[test]
    fn test_parse_dos_skips_dot_entries() {
        let m = manager_at("/");
        assert!(m
            .parse_dos_listing("01-01-25  12:00PM       <DIR>          ..")
            .is_none());
    }

    #[test]
    fn test_trailing_slash_path() {
        let m = manager_at("/home/");
        let file = m
            .parse_unix_listing("-rw-r--r--   1 user group  0 Jan 01 00:00 test.txt")
            .unwrap();
        assert_eq!(file.path, "/home/test.txt");
    }

    #[test]
    fn test_cancellation_flag() {
        let m = FtpManager::new();
        assert!(!m.is_cancelled());
        m.cancel();
        assert!(m.is_cancelled());
        m.reset_cancel();
        assert!(!m.is_cancelled());
    }
}

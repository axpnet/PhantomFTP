//! Application State and Event Handling
//! 
//! This module manages the main application state and handles user events.

#![allow(dead_code)] // Some functions are reserved for future use

use crossterm::event::{KeyEvent, KeyCode, KeyModifiers};
use ratatui::widgets::ListState;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{info, error};

use crate::ftp::{FtpManager, RemoteFile, TransferProgress, ProtocolType};

/// Local file representation
#[derive(Debug, Clone)]
pub struct LocalFile {
    pub name: String,
    pub path: PathBuf,
    pub size: Option<u64>,
    pub is_dir: bool,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    Key(KeyEvent),
    FtpConnected,
    FtpDisconnected,
    FtpFilesListed(Vec<RemoteFile>),
    FtpPathChanged(String),
    FtpError(String),
    TransferProgress(TransferProgress),
    TransferCompleted(String),
    StatusMessage(String),
    FilePreview(String, String), // (filename, content)
}

#[derive(Debug, Clone)]
pub struct ConnectionDialog {
    server: String,
    username: String,
    password: String,
    use_tls: bool,
    protocol: ProtocolType,
    selected_field: usize,
}

impl ConnectionDialog {
    pub fn new() -> Self {
        Self {
            server: String::new(),
            username: String::new(),
            password: String::new(),
            use_tls: false,
            protocol: ProtocolType::Ftp,
            selected_field: 0,
        }
    }
    
    pub fn server(&self) -> &str {
        &self.server
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn password(&self) -> &str {
        &self.password
    }
    
    pub fn use_tls(&self) -> bool {
        self.use_tls
    }
    
    pub fn protocol(&self) -> &ProtocolType {
        &self.protocol
    }

    pub fn selected_field(&self) -> usize {
        self.selected_field
    }

    pub fn is_complete(&self) -> bool {
        !self.server.is_empty() && !self.username.is_empty() && !self.password.is_empty()
    }

    pub fn next_field(&mut self) {
        self.selected_field = (self.selected_field + 1) % 5; // Now we have 5 fields
    }

    pub fn prev_field(&mut self) {
        self.selected_field = (self.selected_field + 4) % 5; // Handle underflow
    }

    pub fn input_char(&mut self, c: char) {
        match self.selected_field {
            0 => self.server.push(c),
            1 => self.username.push(c),
            2 => self.password.push(c),
            3 => {}, // Protocol field is selected with arrows
            4 => {}, // Placeholder for future fields
            _ => {}
        }
    }

    pub fn backspace(&mut self) {
        match self.selected_field {
            0 => { self.server.pop(); }
            1 => { self.username.pop(); }
            2 => { self.password.pop(); }
            3 => {}, // Protocol field is selected with arrows
            4 => {}, // Placeholder for future fields
            _ => {}
        }
    }
    
    pub fn toggle_tls(&mut self) {
        self.use_tls = !self.use_tls;
    }
    
    pub fn select_protocol(&mut self, protocol: ProtocolType) {
        self.protocol = protocol;
    }
    
    pub fn cycle_protocol(&mut self) {
        self.protocol = match self.protocol {
            ProtocolType::Ftp => ProtocolType::Ftps,
            ProtocolType::Ftps => ProtocolType::Ftp,
        };
    }
}

impl Default for ConnectionDialog {
    fn default() -> Self {
        Self::new()
    }
}

pub struct App {
    pub is_connected: bool,
    pub current_local_path_buf: PathBuf,
    pub current_remote_path: String,
    pub local_files: Vec<LocalFile>,
    pub remote_files: Vec<RemoteFile>,
    pub local_list_state: ListState,
    pub remote_list_state: ListState,
    pub selected_local_file: Option<usize>,
    pub selected_remote_file: Option<usize>,
    pub local_focused: bool,
    pub remote_focused: bool,
    pub status_message: String,
    pub error_message: Option<String>,
    pub show_connection_dialog: bool,
    pub show_help: bool,
    pub connection_dialog: ConnectionDialog,
    pub event_sender: Option<mpsc::UnboundedSender<AppEvent>>,
    pub ftp_manager: Arc<Mutex<FtpManager>>,
    pub upload_in_progress: bool,
    pub download_in_progress: bool,
    pub transfer_cancelled: bool,
    pub transfer_progress: Option<TransferProgress>,
    pub show_preview: bool,
    pub preview_content: Option<String>,
    pub preview_file: Option<String>,
}

impl App {
    pub fn new() -> Self {
        Self {
            is_connected: false,
            current_local_path_buf: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            current_remote_path: "/".to_string(),
            local_files: Vec::new(),
            remote_files: Vec::new(),
            local_list_state: ListState::default(),
            remote_list_state: ListState::default(),
            selected_local_file: None,
            selected_remote_file: None,
            local_focused: true,
            remote_focused: false,
            status_message: "Welcome! Press 'c' to connect.".to_string(),
            error_message: None,
            show_connection_dialog: false,
            show_help: false,
            connection_dialog: ConnectionDialog::new(),
            event_sender: None,
            ftp_manager: Arc::new(Mutex::new(FtpManager::new())),
            upload_in_progress: false,
            download_in_progress: false,
            transfer_cancelled: false,
            transfer_progress: None,
            show_preview: false,
            preview_content: None,
            preview_file: None,
        }
    }
    
    /// Check if transfer was cancelled (for use in async transfer loops)
    async fn is_transfer_cancelled(_sender: &Option<mpsc::UnboundedSender<AppEvent>>) -> bool {
        // This is a placeholder - in a real implementation you'd check a cancellation flag
        // For now, always return false (transfer not cancelled)
        false
    }
    
    /// Refresh local file listing
    pub fn refresh_local_files(&mut self) {
        let mut files = Vec::new();
        
        // Add parent directory entry if not at root
        if let Some(parent) = self.current_local_path_buf.parent() {
            files.push(LocalFile {
                name: "..".to_string(),
                path: parent.to_path_buf(),
                size: None,
                is_dir: true,
            });
        }
        
        // Read directory contents
        if let Ok(entries) = std::fs::read_dir(&self.current_local_path_buf) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    
                    // Skip hidden files (starting with .)
                    if name.starts_with('.') {
                        continue;
                    }
                    
                    files.push(LocalFile {
                        name,
                        path: entry.path(),
                        size: if metadata.is_file() { Some(metadata.len()) } else { None },
                        is_dir: metadata.is_dir(),
                    });
                }
            }
        }
        
        // Sort: directories first, then files, both alphabetically
        files.sort_by(|a, b| {
            if a.name == ".." { return std::cmp::Ordering::Less; }
            if b.name == ".." { return std::cmp::Ordering::Greater; }
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });
        
        self.local_files = files;
        self.selected_local_file = if self.local_files.is_empty() { None } else { Some(0) };
        self.local_list_state.select(self.selected_local_file);
    }

    pub fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Key(key) => self.handle_key_event(key),
            AppEvent::FtpConnected => {
                self.status_message = "Connected".to_string();
                self.error_message = None;
                self.is_connected = true;
            }
            AppEvent::FtpDisconnected => {
                self.status_message = "Disconnected".to_string();
                self.remote_files.clear();
                self.is_connected = false;
            }
            AppEvent::FtpFilesListed(files) => {
                self.remote_files = files;
                self.selected_remote_file = if self.remote_files.is_empty() { None } else { Some(0) };
                self.remote_list_state.select(self.selected_remote_file);
                self.status_message = format!("{} items", self.remote_files.len());
            }
            AppEvent::FtpPathChanged(new_path) => {
                self.current_remote_path = new_path;
                self.status_message = format!("Entered: {}", self.current_remote_path);
            }
            AppEvent::FtpError(error) => {
                self.error_message = Some(error.clone());
                self.status_message = error;
                self.upload_in_progress = false;
                self.download_in_progress = false;
            }
            AppEvent::TransferProgress(progress) => {
                self.transfer_progress = Some(progress.clone());
                self.status_message = format!("Transferring {}: {}%", progress.filename, progress.percentage);
                self.download_in_progress = true;
            }
            AppEvent::TransferCompleted(filename) => {
                self.status_message = format!("✓ Transfer completed: {}", filename);
                self.upload_in_progress = false;
                self.download_in_progress = false;
                self.transfer_progress = None;
            }
            AppEvent::StatusMessage(msg) => {
                self.status_message = msg;
            }
            AppEvent::FilePreview(filename, content) => {
                self.preview_file = Some(filename);
                self.preview_content = Some(content);
                self.show_preview = true;
            }

        }
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        if self.show_connection_dialog {
            self.handle_connection_dialog_key(key);
            return;
        }

        match key.code {
            KeyCode::Char('c') | KeyCode::Char('C') => {
                if !key.modifiers.contains(KeyModifiers::CONTROL) && !self.is_connected {
                    self.show_connection_dialog = true;
                } else if key.modifiers.contains(KeyModifiers::CONTROL) && (self.upload_in_progress || self.download_in_progress) {
                    // Ctrl+C to cancel transfer
                    self.transfer_cancelled = true;
                    self.status_message = "Transfer cancelled".to_string();
                    self.upload_in_progress = false;
                    self.download_in_progress = false;
                }
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                if self.is_connected {
                    self.refresh_remote_files();
                }
            }
            KeyCode::Char('u') | KeyCode::Char('U') => {
                if self.is_connected && !self.remote_focused {
                    self.upload_file_prompt();
                }
            }
            KeyCode::Tab => {
                self.remote_focused = !self.remote_focused;
                self.status_message = if self.remote_focused {
                    "Focus: Remote Files".to_string()
                } else {
                    format!("Focus: Local Files ({})", self.current_local_path_buf.display())
                };
            }
            KeyCode::Up => {
                if self.remote_focused {
                    self.move_remote_selection_up();
                } else {
                    self.move_local_selection_up();
                }
            }
            KeyCode::Down => {
                if self.remote_focused {
                    self.move_remote_selection_down();
                } else {
                    self.move_local_selection_down();
                }
            }
            KeyCode::Enter => {
                if self.remote_focused {
                    self.handle_remote_enter();
                } else {
                    self.handle_local_enter();
                }
            }
            KeyCode::Backspace => {
                if self.remote_focused && self.is_connected {
                    self.handle_remote_backspace();
                } else if !self.remote_focused {
                    self.handle_local_backspace();
                }
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                // Debug: manually refresh local files
                if !self.remote_focused {
                    self.refresh_local_files();
                    self.status_message = format!("Refreshed local: {} files", self.local_files.len());
                }
            }
            KeyCode::Char('h') | KeyCode::Char('H') => {
                // Toggle help dialog
                self.show_help = !self.show_help;
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                // Preview functionality temporarily disabled with SFTP removal
                // Will be re-enabled in v1.1 with full SFTP support
                self.status_message = "File preview temporarily disabled".to_string();
            }
            KeyCode::Esc => {
                if self.show_preview {
                    self.show_preview = false;
                    self.preview_content = None;
                    self.preview_file = None;
                } else {
                    self.show_connection_dialog = false;
                    self.show_help = false;
                }
            }
            _ => {}
        }
    }

    fn handle_connection_dialog_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.show_connection_dialog = false;
                self.show_help = false;
            }
            KeyCode::Enter => {
                if self.connection_dialog.is_complete() {
                    self.connect_to_server();
                    self.show_connection_dialog = false;
                }
            }
            KeyCode::Tab => {
                self.connection_dialog.next_field();
            }
            KeyCode::BackTab => {
                self.connection_dialog.prev_field();
            }
            KeyCode::Backspace => {
                self.connection_dialog.backspace();
            }
            KeyCode::Up => {
                if self.connection_dialog.selected_field() == 3 {
                    self.connection_dialog.cycle_protocol();
                } else {
                    self.connection_dialog.prev_field();
                }
            }
            KeyCode::Down => {
                if self.connection_dialog.selected_field() == 3 {
                    self.connection_dialog.cycle_protocol();
                } else {
                    self.connection_dialog.next_field();
                }
            }
            KeyCode::Char(' ') => {
                // Spacebar to toggle TLS option
                if self.connection_dialog.selected_field() == 3 { // Assuming TLS is field 3
                    self.connection_dialog.toggle_tls();
                }
            }
            KeyCode::Char(c) => {
                self.connection_dialog.input_char(c);
            }
            _ => {}
        }
    }

    pub async fn process_events(&mut self) {
        // This method is called in the main loop
    }

    pub fn set_event_sender(&mut self, sender: mpsc::UnboundedSender<AppEvent>) {
        self.event_sender = Some(sender);
    }

    fn send_event(&self, event: AppEvent) {
        if let Some(sender) = &self.event_sender {
            let _ = sender.send(event);
        }
    }

    fn connect_to_server(&mut self) {
        let ftp_manager = Arc::clone(&self.ftp_manager);
        let sender = self.event_sender.clone();
        let server = self.connection_dialog.server().to_string();
        let username = self.connection_dialog.username().to_string();
        let password = self.connection_dialog.password().to_string();
        let protocol = self.connection_dialog.protocol().clone();
        
        self.status_message = format!("Connecting to {}...", server);

        tokio::spawn(async move {
            let mut manager = ftp_manager.lock().await;
            
            // Connect based on protocol type - only FTP/FTPS supported now
            let result = match protocol {
                ProtocolType::Ftp => {
                    match manager.connect(&server).await {
                        Ok(_) => {
                            // Login after connection
                            match manager.login(&username, &password).await {
                                Ok(_) => Ok(()),
                                Err(e) => Err(e),
                            }
                        }
                        Err(e) => Err(e),
                    }
                }
                ProtocolType::Ftps => {
                    // For TLS, we need the hostname for certificate verification
                    let hostname = if server.contains(':') {
                        server.split(':').next().unwrap_or(&server).to_string()
                    } else {
                        server.clone()
                    };
                    
                    match manager.connect_secure(&server, &hostname).await {
                        Ok(_) => {
                            // Login after secure connection
                            match manager.login(&username, &password).await {
                                Ok(_) => Ok(()),
                                Err(e) => Err(e),
                            }
                        }
                        Err(e) => Err(e),
                    }
                }
            };
            
            match result {
                Ok(_) => {
                    info!("Connected to server ({:?}): {}", protocol, server);
                    
                    // Get initial file listing
                    match manager.list_files().await {
                        Ok(files) => {
                            if let Some(s) = &sender {
                                let _ = s.send(AppEvent::FtpConnected);
                                let _ = s.send(AppEvent::FtpFilesListed(files));
                            }
                        }
                        Err(e) => {
                            if let Some(s) = &sender {
                                let _ = s.send(AppEvent::FtpError(format!("Failed to list files: {:?}", e)));
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to connect to server ({:?}): {:?} - {:?}", protocol, server, e);
                    if let Some(s) = &sender {
                        let _ = s.send(AppEvent::FtpError(format!("Connection failed: {:?}", e)));
                    }
                }
            }
        });
    }

    fn refresh_remote_files(&mut self) {
        let ftp_manager = Arc::clone(&self.ftp_manager);
        let sender = self.event_sender.clone();
        
        self.status_message = "Refreshing...".to_string();

        tokio::spawn(async move {
            let mut manager = ftp_manager.lock().await;
            // 获取当前路径
            let current_path = manager.current_path().to_string();
            match manager.list_files().await {
                Ok(files) => {
                    if let Some(s) = &sender {
                        let _ = s.send(AppEvent::FtpFilesListed(files));
                        // 发送路径更新事件
                        let _ = s.send(AppEvent::FtpPathChanged(current_path));
                    }
                }
                Err(e) => {
                    error!("Failed to refresh files: {:?}", e);
                    if let Some(s) = &sender {
                        let _ = s.send(AppEvent::FtpError(format!("Failed to refresh: {:?}", e)));
                    }
                }
            }
        });
    }

    fn handle_remote_enter(&mut self) {
        if let Some(index) = self.selected_remote_file {
            // 先克隆需要的数据，避免借用冲突
            let file_name = self.remote_files.get(index).map(|f| f.name.clone());
            let file_path = self.remote_files.get(index).map(|f| f.path.clone());
            let is_dir = self.remote_files.get(index).map(|f| f.is_dir);
            
            if let (Some(name), Some(path), Some(is_dir)) = (file_name, file_path, is_dir) {
                if is_dir {
                    self.change_remote_directory(&name);
                } else {
                    self.download_file(&path);
                }
            }
        }
    }

    fn handle_remote_backspace(&mut self) {
        let ftp_manager = Arc::clone(&self.ftp_manager);
        let sender = self.event_sender.clone();
        
        self.status_message = "Going to parent directory...".to_string();

        tokio::spawn(async move {
            let mut manager = ftp_manager.lock().await;
            match manager.go_up().await {
                Ok(_) => {
                    // 获取新的当前路径
                    let new_path = manager.current_path().to_string();
                    match manager.list_files().await {
                        Ok(files) => {
                            if let Some(s) = &sender {
                                let _ = s.send(AppEvent::FtpFilesListed(files));
                                // 发送路径更新事件
                                let _ = s.send(AppEvent::FtpPathChanged(new_path));
                            }
                        }
                        Err(e) => {
                            if let Some(s) = &sender {
                                let _ = s.send(AppEvent::FtpError(format!("Failed to list files: {:?}", e)));
                            }
                        }
                    }
                }
                Err(e) => {
                    if let Some(s) = &sender {
                        let _ = s.send(AppEvent::FtpError(format!("Failed to go up: {:?}", e)));
                    }
                }
            }
        });
    }

    fn change_remote_directory(&mut self, dir_name: &str) {
        let ftp_manager = Arc::clone(&self.ftp_manager);
        let sender = self.event_sender.clone();
        let dir_name = dir_name.to_string();
        
        self.status_message = format!("Entering directory: {}", dir_name);

        tokio::spawn(async move {
            let mut manager = ftp_manager.lock().await;
            match manager.change_dir(&dir_name).await {
                Ok(_) => {
                    // 更新当前路径
                    let new_path = manager.current_path().to_string();
                    // 列出新目录中的文件
                    match manager.list_files().await {
                        Ok(files) => {
                            if let Some(s) = &sender {
                                let _ = s.send(AppEvent::FtpFilesListed(files));
                                // 发送路径更新事件
                                let _ = s.send(AppEvent::FtpPathChanged(new_path));
                            }
                        }
                        Err(e) => {
                            if let Some(s) = &sender {
                                let _ = s.send(AppEvent::FtpError(format!("Failed to list files: {:?}", e)));
                            }
                        }
                    }
                }
                Err(e) => {
                    if let Some(s) = &sender {
                        let _ = s.send(AppEvent::FtpError(format!("Failed to change directory: {:?}", e)));
                    }
                }
            }
        });
    }

    fn download_file(&mut self, remote_path: &str) {
        let ftp_manager = Arc::clone(&self.ftp_manager);
        let sender = self.event_sender.clone(); // Clone sender for use in async block
        let remote_path = remote_path.to_string();
        let local_path = self.current_local_path_buf.join(
            std::path::Path::new(&remote_path)
                .file_name()
                .unwrap_or(std::ffi::OsStr::new("downloaded_file"))
        ).to_string_lossy().to_string();
        
        self.download_in_progress = true;
        self.transfer_cancelled = false;
        self.status_message = format!("Downloading: {}", remote_path);

        tokio::spawn(async move {
            // Retry logic - attempt download up to 3 times
            let mut attempts = 0;
            let max_attempts = 3;
            let mut last_error = None;
            
            while attempts < max_attempts && !Self::is_transfer_cancelled(&sender).await {
                attempts += 1;
                if attempts > 1 {
                    // Notify about retry attempt
                    if let Some(s) = &sender {
                        let _ = s.send(AppEvent::StatusMessage(
                            format!("Download attempt {}/{} failed. Retrying in 2 seconds...", 
                                attempts - 1, max_attempts)
                        ));
                    }
                    
                    // Wait before retrying
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
                
                let mut manager = ftp_manager.lock().await;
                let progress_sender = sender.clone();
                let result = manager.download_file(&remote_path, &local_path, move |progress| {
                    if let Some(s) = &progress_sender {
                        let _ = s.send(AppEvent::TransferProgress(progress));
                    }
                }).await;
                
                match result {
                    Ok(_) => {
                        // Download successful
                        if let Some(s) = &sender {
                            let _ = s.send(AppEvent::TransferCompleted(format!("Downloaded: {}", remote_path)));
                        }
                        return;
                    }
                    Err(e) => {
                        last_error = Some(e);
                    }
                }
            }
            
            // All retry attempts exhausted or cancelled
            if let Some(s) = &sender {
                if Self::is_transfer_cancelled(&sender).await {
                    let _ = s.send(AppEvent::FtpError("Transfer cancelled by user".to_string()));
                } else {
                    let _ = s.send(AppEvent::FtpError(
                        format!("Download failed after {} attempts: {:?}", max_attempts, last_error)
                    ));
                }
            }
        });
    }

    fn upload_file(&mut self, local_path: &str) {
        let ftp_manager = Arc::clone(&self.ftp_manager);
        let sender = self.event_sender.clone();
        let local_path = local_path.to_string(); // Clone the path to own it
        let remote_path = std::path::Path::new(&local_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("uploaded_file")
            .to_string();
        
        self.upload_in_progress = true;
        self.transfer_cancelled = false;
        self.status_message = format!("Uploading: {}", remote_path);

        tokio::spawn(async move {
            // Retry logic - attempt upload up to 3 times
            let mut attempts = 0;
            let max_attempts = 3;
            let mut last_error = None;
            
            while attempts < max_attempts && !Self::is_transfer_cancelled(&sender).await {
                attempts += 1;
                if attempts > 1 {
                    // Notify about retry attempt
                    if let Some(s) = &sender {
                        let _ = s.send(AppEvent::StatusMessage(
                            format!("Upload attempt {}/{} failed. Retrying in 2 seconds...", 
                                attempts - 1, max_attempts)
                        ));
                    }
                    
                    // Wait before retrying
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
                
                let mut manager = ftp_manager.lock().await;
                let local_path_clone = local_path.clone(); // Clone for use in closure
                let progress_sender = sender.clone();
                let result = manager.upload_file(&local_path_clone, &remote_path, move |progress| {
                    if let Some(s) = &progress_sender {
                        let _ = s.send(AppEvent::TransferProgress(progress));
                    }
                }).await;
                
                match result {
                    Ok(_) => {
                        // Upload successful
                        if let Some(s) = &sender {
                            let _ = s.send(AppEvent::TransferCompleted(format!("Uploaded: {}", remote_path)));
                        }
                        return;
                    }
                    Err(e) => {
                        last_error = Some(e);
                    }
                }
            }
            
            // All retry attempts exhausted or cancelled
            if let Some(s) = &sender {
                if Self::is_transfer_cancelled(&sender).await {
                    let _ = s.send(AppEvent::FtpError("Transfer cancelled by user".to_string()));
                } else {
                    let _ = s.send(AppEvent::FtpError(
                        format!("Upload failed after {} attempts: {:?}", max_attempts, last_error)
                    ));
                }
            }
        });
    }

    fn upload_file_prompt(&mut self) {
        // For now, we'll just show a message about how to upload
        // In a future implementation, we might add a file picker
        self.status_message = "To upload a file, select it in the local panel and press Enter".to_string();
    }

    fn move_remote_selection_up(&mut self) {
        if self.remote_files.is_empty() {
            return;
        }

        let i = self.selected_remote_file.map(|s| s.saturating_sub(1)).unwrap_or(0);
        self.selected_remote_file = Some(i.min(self.remote_files.len() - 1));
        self.remote_list_state.select(self.selected_remote_file);
    }

    fn move_remote_selection_down(&mut self) {
        if self.remote_files.is_empty() {
            return;
        }

        let i = self.selected_remote_file.map(|s| s + 1).unwrap_or(0);
        self.selected_remote_file = Some(i.min(self.remote_files.len() - 1));
        self.remote_list_state.select(self.selected_remote_file);
    }

    fn move_local_selection_up(&mut self) {
        if self.local_files.is_empty() {
            return;
        }
        let i = self.selected_local_file.map(|s| s.saturating_sub(1)).unwrap_or(0);
        self.selected_local_file = Some(i.min(self.local_files.len() - 1));
        self.local_list_state.select(self.selected_local_file);
    }

    fn move_local_selection_down(&mut self) {
        if self.local_files.is_empty() {
            return;
        }
        let i = self.selected_local_file.map(|s| s + 1).unwrap_or(0);
        self.selected_local_file = Some(i.min(self.local_files.len() - 1));
        self.local_list_state.select(self.selected_local_file);
    }
    
    fn handle_local_enter(&mut self) {
        if let Some(index) = self.selected_local_file {
            if let Some(file) = self.local_files.get(index).cloned() {
                if file.is_dir {
                    self.current_local_path_buf = file.path;
                    self.refresh_local_files();
                    self.status_message = format!("Entered: {}", self.current_local_path_buf.display());
                } else if self.is_connected && !self.remote_focused {
                    // Upload file when pressing Enter on a local file and remote panel is not focused
                    self.upload_file(&file.path.to_string_lossy());
                } else {
                    self.status_message = format!("Selected file: {} (not a dir)", file.name);
                }
            } else {
                self.status_message = format!("No file at index {}", index);
            }
        } else {
            self.status_message = "No file selected".to_string();
        }
    }
    
    fn handle_local_backspace(&mut self) {
        if let Some(parent) = self.current_local_path_buf.parent() {
            self.current_local_path_buf = parent.to_path_buf();
            self.refresh_local_files();
            self.status_message = format!("Local: {}", self.current_local_path_buf.display());
        }
    }

    fn preview_file(&mut self, remote_path: &str) {
        let ftp_manager = Arc::clone(&self.ftp_manager);
        let sender = self.event_sender.clone();
        let remote_path = remote_path.to_string();
        
        // Extract filename for display
        let filename = std::path::Path::new(&remote_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("preview")
            .to_string();
        
        self.status_message = format!("Loading preview for: {}", filename);

        tokio::spawn(async move {
            let mut manager = ftp_manager.lock().await;
            
            match manager.preview_file(&remote_path).await {
                Ok(content) => {
                    if let Some(s) = &sender {
                        let _ = s.send(AppEvent::FilePreview(filename, content));
                    }
                }
                Err(e) => {
                    if let Some(s) = &sender {
                        let _ = s.send(AppEvent::FtpError(format!("Failed to preview file: {:?}", e)));
                    }
                }
            }
        });
    }

    fn enter_local_directory(&mut self) {
        if let Some(selected) = self.selected_local_file {
            let path_to_change = if let Some(file) = self.local_files.get(selected) {
                if file.is_dir {
                    Some(file.path.clone())
                } else {
                    None
                }
            } else {
                None
            };
            
            if let Some(path) = path_to_change {
                self.change_local_directory(&path);
            }
        }
    }

    fn change_local_directory(&mut self, path: &Path) {
        if let Ok(new_path) = std::fs::canonicalize(path) {
            self.current_local_path_buf = new_path.clone();
            self.refresh_local_files();
            
            // Notify about path change
            if let Some(sender) = &self.event_sender {
                // Convert path to string for the event
                let path_str = new_path.to_string_lossy().to_string();
                let _ = sender.send(AppEvent::FtpPathChanged(path_str));
            }
        }
    }

    fn enter_remote_directory(&mut self) {
        if let Some(selected) = self.selected_remote_file {
            if let Some(file) = self.remote_files.get(selected) {
                if file.is_dir {
                    let ftp_manager = Arc::clone(&self.ftp_manager);
                    let sender = self.event_sender.clone();
                    let dir_path = file.path.clone();
                    
                    self.status_message = format!("Entering directory: {}", file.name);
                    
                    tokio::spawn(async move {
                        let mut manager = ftp_manager.lock().await;
                        
                        match manager.change_working_dir(&dir_path).await {
                            Ok(new_path) => {
                                match manager.list_files().await {
                                    Ok(files) => {
                                        if let Some(s) = &sender {
                                            // Convert path to string for the event
                                            let path_str = new_path.clone();
                                            let _ = s.send(AppEvent::FtpPathChanged(path_str));
                                            let _ = s.send(AppEvent::FtpFilesListed(files));
                                        }
                                    }
                                    Err(e) => {
                                        if let Some(s) = &sender {
                                            let _ = s.send(AppEvent::FtpError(format!("Failed to list files: {:?}", e)));
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                if let Some(s) = &sender {
                                    let _ = s.send(AppEvent::FtpError(format!("Failed to change directory: {:?}", e)));
                                }
                            }
                        }
                    });
                }
            }
        }
    }

    fn go_to_parent_remote_directory(&mut self) {
        let ftp_manager = Arc::clone(&self.ftp_manager);
        let sender = self.event_sender.clone();
        
        self.status_message = "Going to parent directory...".to_string();
        
        tokio::spawn(async move {
            let mut manager = ftp_manager.lock().await;
            
            match manager.cd_up().await {
                Ok(new_path) => {
                    match manager.list_files().await {
                        Ok(files) => {
                            if let Some(s) = &sender {
                                // Convert path to string for the event
                                let path_str = new_path.clone();
                                let _ = s.send(AppEvent::FtpPathChanged(path_str));
                                let _ = s.send(AppEvent::FtpFilesListed(files));
                            }
                        }
                        Err(e) => {
                            if let Some(s) = &sender {
                                let _ = s.send(AppEvent::FtpError(format!("Failed to list files: {:?}", e)));
                            }
                        }
                    }
                }
                Err(e) => {
                    if let Some(s) = &sender {
                        let _ = s.send(AppEvent::FtpError(format!("Failed to change directory: {:?}", e)));
                    }
                }
            }
        });
    }

    // Public getter methods
    pub fn remote_files(&self) -> &[RemoteFile] {
        &self.remote_files
    }


    pub fn current_local_path(&self) -> String {
        self.current_local_path_buf.to_string_lossy().to_string()
    }

    pub fn current_remote_path(&self) -> &str {
        &self.current_remote_path
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected
    }

    pub fn is_remote_focused(&self) -> bool {
        self.remote_focused
    }

    pub fn selected_remote_file(&self) -> Option<usize> {
        self.selected_remote_file
    }

    pub fn selected_local_file(&self) -> Option<usize> {
        self.selected_local_file
    }

    pub fn show_connection_dialog(&self) -> bool {
        self.show_connection_dialog
    }

    pub fn connection_dialog(&self) -> &ConnectionDialog {
        &self.connection_dialog
    }

    pub fn remote_list_state(&mut self) -> &mut ListState {
        &mut self.remote_list_state
    }

    pub fn local_list_state(&mut self) -> &mut ListState {
        &mut self.local_list_state
    }

    pub fn can_quit(&self) -> bool {
        !self.show_connection_dialog
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
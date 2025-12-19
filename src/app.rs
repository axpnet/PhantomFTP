//! Application State and Event Handling
//! 
//! This module manages the main application state and handles user events.

use crossterm::event::{KeyEvent, KeyCode, KeyModifiers};
use ratatui::widgets::ListState;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{info, error};

use crate::ftp::{FtpManager, RemoteFile};

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
    FtpError(String),
    StatusMessage(String),
}

#[derive(Debug, Clone)]
pub struct ConnectionDialog {
    pub server: String,
    pub username: String,
    pub password: String,
    pub selected_field: usize,
}

impl ConnectionDialog {
    pub fn new() -> Self {
        Self {
            server: String::new(),
            username: String::new(),
            password: String::new(),
            selected_field: 0,
        }
    }

    pub fn server(&self) -> &str { &self.server }
    pub fn username(&self) -> &str { &self.username }
    pub fn password(&self) -> &str { &self.password }
    pub fn selected_field(&self) -> usize { self.selected_field }

    pub fn password_mask(&self) -> String {
        "*".repeat(self.password.len())
    }

    pub fn next_field(&mut self) {
        self.selected_field = (self.selected_field + 1) % 3;
    }

    pub fn prev_field(&mut self) {
        self.selected_field = (self.selected_field + 2) % 3;
    }

    pub fn input_char(&mut self, c: char) {
        match self.selected_field {
            0 => self.server.push(c),
            1 => self.username.push(c),
            2 => self.password.push(c),
            _ => {},
        }
    }

    pub fn backspace(&mut self) {
        match self.selected_field {
            0 => { self.server.pop(); },
            1 => { self.username.pop(); },
            2 => { self.password.pop(); },
            _ => {},
        }
    }

    pub fn is_complete(&self) -> bool {
        !self.server.is_empty() && !self.username.is_empty() && !self.password.is_empty()
    }
}

impl Default for ConnectionDialog {
    fn default() -> Self {
        Self::new()
    }
}

pub struct App {
    ftp_manager: Arc<Mutex<FtpManager>>,
    pub remote_files: Vec<RemoteFile>,
    pub local_files: Vec<LocalFile>,
    pub remote_list_state: ListState,
    pub local_list_state: ListState,
    pub selected_remote_file: Option<usize>,
    pub selected_local_file: Option<usize>,
    pub remote_focused: bool,
    pub current_tab: usize,
    pub show_connection_dialog: bool,
    pub connection_dialog: ConnectionDialog,
    pub status_message: String,
    pub error_message: Option<String>,
    pub is_connected: bool,
    current_remote_path: String,
    pub current_local_path_buf: PathBuf,
    event_sender: Option<mpsc::UnboundedSender<AppEvent>>,
}

impl App {
    pub fn new(server: Option<String>, username: Option<String>, password: Option<String>) -> Self {
        let mut connection_dialog = ConnectionDialog::new();
        
        if let Some(s) = server { connection_dialog.server = s; }
        if let Some(u) = username { connection_dialog.username = u; }
        if let Some(p) = password { connection_dialog.password = p; }

        // Get home directory as starting point for local files
        let home_dir = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/"));

        let mut app = Self {
            ftp_manager: Arc::new(Mutex::new(FtpManager::new())),
            remote_files: Vec::new(),
            local_files: Vec::new(),
            remote_list_state: ListState::default(),
            local_list_state: ListState::default(),
            selected_remote_file: None,
            selected_local_file: Some(0),
            remote_focused: true,
            current_tab: 0,
            show_connection_dialog: false,
            connection_dialog,
            status_message: "Ready - Press 'c' to connect".to_string(),
            error_message: None,
            is_connected: false,
            current_remote_path: "/".to_string(),
            current_local_path_buf: home_dir,
            event_sender: None,
        };
        
        // Load local files on startup
        app.refresh_local_files();
        app
    }
    
    /// Refresh local file listing
    pub fn refresh_local_files(&mut self) {
        let mut files = Vec::new();
        
        // Add parent directory entry if not at root
        if self.current_local_path_buf.parent().is_some() {
            files.push(LocalFile {
                name: "..".to_string(),
                path: self.current_local_path_buf.parent().unwrap().to_path_buf(),
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
            AppEvent::FtpError(error) => {
                self.error_message = Some(error.clone());
                self.status_message = error;
            }
            AppEvent::StatusMessage(msg) => {
                self.status_message = msg;
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
            _ => {}
        }
    }

    fn handle_connection_dialog_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.show_connection_dialog = false;
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
                self.connection_dialog.prev_field();
            }
            KeyCode::Down => {
                self.connection_dialog.next_field();
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
        let server = self.connection_dialog.server.clone();
        let username = self.connection_dialog.username.clone();
        let password = self.connection_dialog.password.clone();
        let ftp_manager = Arc::clone(&self.ftp_manager);
        let sender = self.event_sender.clone();

        self.status_message = format!("Connecting to {}...", server);

        tokio::spawn(async move {
            let mut manager = ftp_manager.lock().await;
            
            // Connect to server
            match manager.connect(&server).await {
                Ok(_) => {
                    info!("Connected to {}", server);
                    
                    // Login
                    match manager.login(&username, &password).await {
                        Ok(_) => {
                            info!("Logged in as {}", username);
                            if let Some(s) = &sender {
                                let _ = s.send(AppEvent::FtpConnected);
                            }
                            
                            // List files
                            match manager.list_files().await {
                                Ok(files) => {
                                    if let Some(s) = &sender {
                                        let _ = s.send(AppEvent::FtpFilesListed(files));
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to list files: {:?}", e);
                                    if let Some(s) = &sender {
                                        let _ = s.send(AppEvent::FtpError(format!("Failed to list files: {:?}", e)));
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Login failed: {:?}", e);
                            if let Some(s) = &sender {
                                let _ = s.send(AppEvent::FtpError(format!("Login failed: {:?}", e)));
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Connection failed: {:?}", e);
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
            match manager.list_files().await {
                Ok(files) => {
                    if let Some(s) = &sender {
                        let _ = s.send(AppEvent::FtpFilesListed(files));
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
            if let Some(file) = self.remote_files.get(index).cloned() {
                if file.is_dir {
                    self.change_remote_directory(&file.name);
                } else {
                    self.download_file(&file.path);
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
                    match manager.list_files().await {
                        Ok(files) => {
                            if let Some(s) = &sender {
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
                    match manager.list_files().await {
                        Ok(files) => {
                            if let Some(s) = &sender {
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

    fn download_file(&mut self, remote_path: &str) {
        let ftp_manager = Arc::clone(&self.ftp_manager);
        let sender = self.event_sender.clone();
        let remote_path = remote_path.to_string();
        
        let _ = std::fs::create_dir_all("./downloads");
        
        // Extract filename before moving remote_path
        let filename = std::path::Path::new(&remote_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("download")
            .to_string();
        let local_path = format!("./downloads/{}", filename);
        
        self.status_message = format!("Downloading: {}", remote_path);

        tokio::spawn(async move {
            let mut manager = ftp_manager.lock().await;
            match manager.download_file(&remote_path, &local_path).await {
                Ok(_) => {
                    info!("Downloaded: {} -> {}", remote_path, local_path);
                    if let Some(s) = &sender {
                        let _ = s.send(AppEvent::StatusMessage(format!("✓ Downloaded: {}", filename)));
                    }
                }
                Err(e) => {
                    error!("Download failed: {:?}", e);
                    if let Some(s) = &sender {
                        let _ = s.send(AppEvent::FtpError(format!("Download failed: {:?}", e)));
                    }
                }
            }
        });
    }

    fn upload_file_prompt(&mut self) {
        self.status_message = "Upload: Create ./test_upload.txt first".to_string();
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

    // Public getter methods
    pub fn remote_files(&self) -> &[RemoteFile] {
        &self.remote_files
    }

    pub fn current_remote_path(&self) -> &str {
        &self.current_remote_path
    }

    pub fn current_local_path(&self) -> String {
        self.current_local_path_buf.to_string_lossy().to_string()
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

    pub fn get_current_tab(&self) -> usize {
        self.current_tab
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
        Self::new(None, None, None)
    }
}
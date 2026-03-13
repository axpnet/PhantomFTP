//! PhantomFTP - CyberPunk TUI FTP/FTPS Client Library
//!
//! A modern, asynchronous FTP client with a TUI interface.

// Library crate: public API items are used by binary and examples,
// not all are used within the library itself.
#![allow(dead_code)]

pub mod app;
pub mod banner;
pub mod config;
pub mod ftp;
pub mod import;
pub mod ui;

pub use app::{App, AppEvent, ConnectionDialog, LocalFile};
pub use config::{AppSettings, Config, ServerConfig, ThemeConfig};
pub use ftp::{FtpManager, FtpManagerError, ProtocolType, RemoteFile, TransferProgress};

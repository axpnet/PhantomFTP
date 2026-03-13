//! FTP Test Suite
//!
//! Tests FTP connectivity independently of the TUI.

use anyhow::Result;
use phantomftp::ftp::{FtpManager, RemoteFile};
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    info!("PhantomFTP - FTP Test Suite");
    info!("===========================");

    let mut ftp = FtpManager::new();

    // Test 1: Connection
    info!("Test 1: Connecting to ftp.dlptest.com...");
    ftp.connect("ftp.dlptest.com:21").await?;
    info!("Connected");

    // Test 2: Login
    info!("Test 2: Logging in...");
    ftp.login("dlpuser", "rNrKYTX9g7z3RgJRmxWuGHbeu").await?;
    info!("Logged in");

    // Test 3: PWD
    info!("Test 3: Getting current directory...");
    match ftp.pwd().await {
        Ok(path) => info!("Current directory: {}", path),
        Err(e) => warn!("Could not get directory: {:?}", e),
    }

    // Test 4: List files
    info!("Test 4: Listing files...");
    match ftp.list_files().await {
        Ok(files) => {
            info!("Found {} items:", files.len());
            display_file_list(&files);
        }
        Err(e) => error!("Failed to list files: {:?}", e),
    }

    // Test 5: Disconnect
    info!("Test 5: Disconnecting...");
    ftp.disconnect().await?;
    info!("Done");

    Ok(())
}

fn display_file_list(files: &[RemoteFile]) {
    for (i, file) in files.iter().enumerate().take(10) {
        let file_type = if file.is_dir { "DIR" } else { "FILE" };
        let size = file
            .size
            .map(format_file_size)
            .unwrap_or_else(|| "-".to_string());
        info!("  {}. [{}] {} ({})", i + 1, file_type, file.name, size);
    }
    if files.len() > 10 {
        info!("  ... and {} more items", files.len() - 10);
    }
}

fn format_file_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    if unit_index == 0 {
        format!("{} B", bytes)
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

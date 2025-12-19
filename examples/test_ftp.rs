//! FTP Test Suite
//! 
//! This example tests the FTP functionality independently of the TUI.

use anyhow::Result;
use rust_ftp_tui::ftp::{FtpManager, RemoteFile};
use std::time::Duration;
use tracing::{error, info, warn};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with more detailed output
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .init();

    info!("========================================");
    info!("  Rust FTP TUI Client - FTP Test Suite");
    info!("========================================");
    info!("");

    // Test server configuration
    let test_servers = vec![
        TestServer {
            name: "DLP Test Server",
            host: "ftp.dlptest.com:21",
            username: "dlpuser",
            password: "rNrKYTX9g7z3RgJRmxWuGHbeu",
            description: "Public test FTP server",
        },
    ];

    // Try each test server
    for server in test_servers {
        info!("Testing: {}", server.name);
        info!("Description: {}", server.description);
        info!("Host: {}", server.host);
        info!("----------------------------------------");

        match test_ftp_server(&server).await {
            Ok(_) => info!("✅ Test completed successfully for {}", server.name),
            Err(e) => {
                error!("❌ Test failed for {}: {:?}", server.name, e);
                info!("Trying next server...");
            }
        }
        info!("");
        sleep(Duration::from_secs(2)).await;
    }

    info!("========================================");
    info!("  FTP Test Suite Completed");
    info!("========================================");

    Ok(())
}

#[derive(Debug)]
struct TestServer {
    name: &'static str,
    host: &'static str,
    username: &'static str,
    password: &'static str,
    description: &'static str,
}

async fn test_ftp_server(server: &TestServer) -> Result<()> {
    let mut ftp_manager = FtpManager::new();

    // Test 1: Connection
    info!("🔌 Test 1: Connecting to FTP server...");
    match ftp_manager.connect(server.host).await {
        Ok(_) => info!("✅ Connected successfully to {}", server.host),
        Err(e) => {
            error!("❌ Connection failed: {:?}", e);
            return Err(e);
        }
    }

    // Test 2: Authentication
    info!("🔐 Test 2: Logging in...");
    match ftp_manager.login(server.username, server.password).await {
        Ok(_) => info!("✅ Logged in successfully as {}", server.username),
        Err(e) => {
            error!("❌ Login failed: {:?}", e);
            ftp_manager.disconnect().await?;
            return Err(e);
        }
    }

    // Test 3: Get current directory
    info!("📍 Test 3: Getting current working directory...");
    match ftp_manager.pwd().await {
        Ok(path) => info!("✅ Current directory: {}", path),
        Err(e) => warn!("⚠️  Could not get current directory: {:?}", e),
    }

    // Test 4: List files
    info!("📋 Test 4: Listing files in current directory...");
    match ftp_manager.list_files().await {
        Ok(files) => {
            info!("✅ Found {} files/directories:", files.len());
            display_file_list(&files);
        }
        Err(e) => {
            error!("❌ Failed to list files: {:?}", e);
        }
    }

    // Test 5: Disconnect
    info!("🔌 Test 5: Disconnecting from server...");
    match ftp_manager.disconnect().await {
        Ok(_) => info!("✅ Disconnected successfully"),
        Err(e) => warn!("⚠️  Error during disconnect: {:?}", e),
    }

    Ok(())
}

fn display_file_list(files: &[RemoteFile]) {
    for (i, file) in files.iter().enumerate().take(10) {
        let file_type = if file.is_dir { "DIR" } else { "FILE" };
        let size = file.size.map(|s| format_file_size(s)).unwrap_or_else(|| "-".to_string());
        
        info!("   {}. [{}] {} ({})", 
            i + 1,
            file_type,
            file.name,
            size
        );
    }
    if files.len() > 10 {
        info!("   ... and {} more items", files.len() - 10);
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
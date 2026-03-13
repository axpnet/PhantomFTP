//! AeroFTP Server Import
//!
//! Imports saved FTP/FTPS servers from AeroFTP's exported JSON format.
//! Only servers with supported protocols (FTP, FTPS) are imported;
//! all other protocols (SFTP, WebDAV, S3, etc.) are silently skipped.

use crate::config::{Config, ServerConfig};
use crate::ftp::ProtocolType;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;
use tracing::info;

/// AeroFTP's ServerProfile format (subset of fields we need)
#[derive(Debug, Deserialize)]
struct AeroFtpServer {
    /// Server display name
    #[serde(default)]
    name: String,

    /// Hostname or IP
    #[serde(default)]
    host: String,

    /// Port number
    #[serde(default = "default_port")]
    port: u16,

    /// Username
    #[serde(default)]
    username: String,

    /// Protocol type string: "ftp", "ftps", "sftp", "s3", "webdav", etc.
    #[serde(default)]
    protocol: String,

    /// Initial remote path
    #[serde(default)]
    #[serde(rename = "initialPath")]
    initial_path: Option<String>,
}

fn default_port() -> u16 {
    21
}

/// Import servers from an AeroFTP export JSON file.
///
/// Reads the file, parses the JSON array of server profiles,
/// filters to FTP/FTPS only, and merges into the existing config.
/// Returns the number of servers imported.
pub fn import_aeroftp_servers(file_path: &Path, config: &mut Config) -> Result<usize> {
    let content = std::fs::read_to_string(file_path)
        .context("Failed to read AeroFTP export file")?;

    let servers: Vec<AeroFtpServer> = serde_json::from_str(&content)
        .context("Failed to parse AeroFTP export JSON")?;

    let mut imported = 0;

    for server in servers {
        let protocol_lower = server.protocol.to_lowercase();

        // Only import FTP and FTPS servers
        let protocol = match protocol_lower.as_str() {
            "ftp" => ProtocolType::Ftp,
            "ftps" => ProtocolType::Ftps,
            _ => {
                info!("Skipping unsupported protocol '{}': {}", server.protocol, server.name);
                continue;
            }
        };

        // Skip if a server with the same host+port+username already exists
        let already_exists = config.servers.iter().any(|s| {
            s.host == server.host && s.port == server.port && s.username == server.username
        });

        if already_exists {
            info!("Skipping duplicate server: {} ({})", server.name, server.host);
            continue;
        }

        let name = if server.name.is_empty() {
            server.host.clone()
        } else {
            server.name
        };

        let port = if server.port == 0 {
            match protocol {
                ProtocolType::Ftp => 21,
                ProtocolType::Ftps => 21,
            }
        } else {
            server.port
        };

        let use_tls = protocol == ProtocolType::Ftps;
        let tls_hostname = if use_tls {
            Some(server.host.clone())
        } else {
            None
        };

        let server_config = ServerConfig {
            name,
            host: server.host,
            username: server.username,
            password: None,
            port,
            protocol,
            use_tls,
            tls_hostname,
            passive_mode: true,
            timeout: 30,
        };

        config.add_server(server_config);
        imported += 1;
    }

    info!("Imported {} server(s) from AeroFTP export", imported);
    Ok(imported)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn make_temp_json(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn test_import_ftp_servers() {
        let json = r#"[
            {"name": "My FTP", "host": "ftp.example.com", "port": 21, "username": "user", "protocol": "ftp"},
            {"name": "Secure", "host": "ftps.example.com", "port": 990, "username": "admin", "protocol": "ftps"}
        ]"#;

        let file = make_temp_json(json);
        let mut config = Config::new();

        let count = import_aeroftp_servers(file.path(), &mut config).unwrap();
        assert_eq!(count, 2);
        assert_eq!(config.servers.len(), 2);

        assert_eq!(config.servers[0].name, "My FTP");
        assert_eq!(config.servers[0].protocol, ProtocolType::Ftp);
        assert!(!config.servers[0].use_tls);

        assert_eq!(config.servers[1].name, "Secure");
        assert_eq!(config.servers[1].protocol, ProtocolType::Ftps);
        assert!(config.servers[1].use_tls);
        assert_eq!(config.servers[1].port, 990);
    }

    #[test]
    fn test_skips_unsupported_protocols() {
        let json = r#"[
            {"name": "FTP", "host": "ftp.test.com", "port": 21, "username": "u", "protocol": "ftp"},
            {"name": "SSH", "host": "ssh.test.com", "port": 22, "username": "u", "protocol": "sftp"},
            {"name": "S3", "host": "s3.test.com", "port": 443, "username": "key", "protocol": "s3"},
            {"name": "WebDAV", "host": "dav.test.com", "port": 443, "username": "u", "protocol": "webdav"}
        ]"#;

        let file = make_temp_json(json);
        let mut config = Config::new();

        let count = import_aeroftp_servers(file.path(), &mut config).unwrap();
        assert_eq!(count, 1);
        assert_eq!(config.servers[0].name, "FTP");
    }

    #[test]
    fn test_skips_duplicates() {
        let json = r#"[
            {"name": "Server", "host": "ftp.test.com", "port": 21, "username": "user", "protocol": "ftp"}
        ]"#;

        let file = make_temp_json(json);
        let mut config = Config::new();
        config.add_server(ServerConfig::new(
            "Existing".to_string(),
            "ftp.test.com".to_string(),
            "user".to_string(),
        ));

        let count = import_aeroftp_servers(file.path(), &mut config).unwrap();
        assert_eq!(count, 0);
        assert_eq!(config.servers.len(), 1);
    }

    #[test]
    fn test_empty_name_uses_host() {
        let json = r#"[
            {"name": "", "host": "ftp.noname.com", "port": 21, "username": "u", "protocol": "ftp"}
        ]"#;

        let file = make_temp_json(json);
        let mut config = Config::new();

        import_aeroftp_servers(file.path(), &mut config).unwrap();
        assert_eq!(config.servers[0].name, "ftp.noname.com");
    }

    #[test]
    fn test_empty_array() {
        let file = make_temp_json("[]");
        let mut config = Config::new();
        let count = import_aeroftp_servers(file.path(), &mut config).unwrap();
        assert_eq!(count, 0);
    }
}

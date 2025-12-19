# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2025-12-19

### Added
- Initial release of Rust FTP TUI Client
- Asynchronous FTP connection using Tokio runtime
- Username/password authentication
- File listing with Unix and DOS format parsing
- Directory navigation (enter directory, go to parent)
- File download functionality
- File upload functionality (basic)
- Dual-panel TUI layout (remote/local)
- Connection dialog with server, username, password fields
- Status bar showing connection state
- Keyboard navigation with intuitive shortcuts
- JSON-based configuration system
- Theme support with customizable colors
- Command-line argument support (--server, --username, --password)

### Technical
- Built with Rust 2021 edition
- Uses ratatui 0.29 for TUI rendering
- Uses suppaftp 7.0 for FTP protocol
- Uses crossterm 0.28 for terminal handling
- Async-first architecture with tokio::sync::Mutex
- Comprehensive error handling with anyhow/thiserror

### Known Limitations
- Local file browser shows placeholder data
- No progress bars for file transfers yet
- No FTPS/SFTP support yet

[Unreleased]: https://github.com/axpnet/rust-ftp-tui/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/axpnet/rust-ftp-tui/releases/tag/v0.1.0

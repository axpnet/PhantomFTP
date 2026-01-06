# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.2] - 2026-01-06

### Fixed
- Version bump for release

## [1.0.1] - 2026-01-06

### Fixed
- GitHub Actions workflow: direct release upload (fixes artifact storage quota)
- Fixed deprecated `dtolnay/rust-action` → `dtolnay/rust-toolchain@stable`
- Removed unused imports and fixed clippy warnings

### Changed
- Merged .deb build into main workflow job
- Simplified CI/CD pipeline

## [1.0.0] - 2025-12-20

### Added
- Full FTPS (FTP over TLS) support
- Progress bar for downloads
- Spinner animation for uploads
- Retry mechanism for failed transfers (3 attempts)
- Transfer cancellation with Ctrl+C
- File preview with 'p' key
- In-app help dialog with 'h' key
- Real local file browser

### Fixed
- Local navigation (Enter/Backspace)
- Remote navigation and directory listing
- Connection dialog with protocol selection
- Dual-panel file browser layout

### Technical
- Async-first architecture with tokio
- Uses ratatui 0.29 for TUI rendering
- Uses suppaftp 7.0 with native-tls for FTP/FTPS

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

[Unreleased]: https://github.com/axpnet/PhantomFTP/compare/v1.0.1...HEAD
[1.0.1]: https://github.com/axpnet/PhantomFTP/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/axpnet/PhantomFTP/compare/v0.1.0...v1.0.0
[0.1.0]: https://github.com/axpnet/PhantomFTP/releases/tag/v0.1.0

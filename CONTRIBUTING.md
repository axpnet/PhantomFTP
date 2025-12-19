# Contributing to Rust FTP TUI Client

First off, thank you for considering contributing to Rust FTP TUI Client! 🎉

## Code of Conduct

This project and everyone participating in it is governed by our commitment to creating a welcoming environment. Please be respectful and constructive in all interactions.

## How Can I Contribute?

### Reporting Bugs

Before creating bug reports, please check existing issues to avoid duplicates.

When creating a bug report, please include:

- **Clear title** describing the issue
- **Steps to reproduce** the behavior
- **Expected behavior** vs what actually happened
- **Screenshots** if applicable
- **Environment info**: OS version, Rust version, terminal emulator

### Suggesting Enhancements

Enhancement suggestions are tracked as GitHub issues.

When suggesting an enhancement:

- **Use a clear title** describing the suggestion
- **Provide detailed description** of the proposed functionality
- **Explain why** this enhancement would be useful
- **List any alternatives** you've considered

### Pull Requests

1. **Fork** the repo and create your branch from `main`
2. **Run tests** to ensure nothing is broken: `cargo test`
3. **Run clippy**: `cargo clippy -- -D warnings`
4. **Format code**: `cargo fmt`
5. **Update documentation** if you changed APIs
6. **Write clear commit messages**

## Development Setup

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/rust-ftp-tui.git
cd rust-ftp-tui

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build the project
cargo build

# Run tests
cargo test

# Run the application
cargo run
```

## Style Guide

### Rust Code

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` before committing
- Use `cargo clippy` to catch common mistakes
- Write documentation comments for public APIs
- Use meaningful variable and function names

### Commit Messages

- Use the present tense ("Add feature" not "Added feature")
- Use the imperative mood ("Move cursor to..." not "Moves cursor to...")
- Limit the first line to 72 characters
- Reference issues and pull requests after the first line

Example:
```
Add progress bar for file downloads

- Implement TransferProgress struct
- Add progress widget to UI
- Update download_file to report progress

Fixes #42
```

### Documentation

- Update README.md for user-facing changes
- Update CHANGELOG.md following Keep a Changelog format
- Add inline documentation for complex code

## Project Structure

```
src/
├── main.rs    - Entry point, terminal setup, main loop
├── app.rs     - Application state and event handling
├── ftp.rs     - FTP operations wrapper
├── ui.rs      - TUI rendering with ratatui
├── config.rs  - Configuration management
└── lib.rs     - Library exports for examples
```

## Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run a specific test
cargo test test_config_creation

# Test FTP functionality (requires network)
cargo run --example test_ftp
```

## Questions?

Feel free to open an issue with the "question" label if you have any questions about contributing.

Thank you for your contribution! 🦀

# 🌐 Rust FTP TUI Client

[![Build](https://github.com/axpnet/rust-ftp-tui/actions/workflows/build.yml/badge.svg)](https://github.com/axpnet/rust-ftp-tui/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20Windows%20%7C%20macOS-blue.svg)](https://github.com/axpnet/rust-ftp-tui/releases)

A modern, asynchronous **Terminal User Interface (TUI)** FTP client built in Rust for Linux/Ubuntu.

> **🤖 AI-Assisted Development Project**
> - **Lead Developer & Supervisor:** axpdev
> - **Architect & Tech Lead:** Gemini 3 Pro (AI)
> - **Initial Execution:** KIMI K2 (AI)  
> - **Refinement & Finalization:** Claude Opus 4.5 via Antigravity (AI)

![Demo Screenshot](docs/screenshot.png)

## ✨ Features

- 🚀 **Asynchronous Operations** - Built on Tokio for non-blocking network operations
- 🖥️ **Beautiful TUI** - Clean terminal interface using Ratatui
- 📁 **Dual Panel Layout** - Remote and local file browsing side by side
- 🔐 **FTP Authentication** - Username/password login support
- 📥 **File Downloads** - Download files with visual feedback
- 📤 **File Uploads** - Upload local files to remote server
- 📂 **Directory Navigation** - Browse directories, go back to parent
- ⚙️ **Configuration** - JSON-based configuration for saved servers
- 🎨 **Themeable** - Customizable color themes

## 🛠️ Installation

### From Source (All Platforms)

```bash
# Clone the repository
git clone https://github.com/axpnet/rust-ftp-tui.git
cd rust-ftp-tui

# Build release version
cargo build --release

# Run the application
# Linux/macOS:
./target/release/rust-ftp-tui
# Windows:
.\target\release\rust-ftp-tui.exe
```

### From .deb Package (Ubuntu/Debian)

```bash
# Download the latest .deb from releases
sudo dpkg -i rust-ftp-tui_0.1.0_amd64.deb

# Run
rust-ftp-tui
```

### Windows Executable

Download `rust-ftp-tui-windows-amd64.exe` from the [Releases](https://github.com/axpnet/rust-ftp-tui/releases) page and run it directly. No installation required!

### Prerequisites

- **Rust** 1.75+ (install via [rustup](https://rustup.rs/)) - only if building from source
- **Supported Platforms**: Linux, Windows 10+, macOS

## 🚀 Usage

### Interactive Mode

```bash
rust-ftp-tui
```

Then press `c` to open the connection dialog and enter your FTP credentials.

### Command Line Arguments

```bash
rust-ftp-tui --server ftp.example.com:21 --username myuser --password mypass
```

### Available Options

| Option       | Short | Description                                   |
| ------------ | ----- | --------------------------------------------- |
| `--server`   | `-s`  | FTP server address (e.g., ftp.example.com:21) |
| `--username` | `-u`  | Username for authentication                   |
| `--password` | `-p`  | Password for authentication                   |
| `--help`     | `-h`  | Show help message                             |
| `--version`  | `-V`  | Show version                                  |

## ⌨️ Key Bindings

| Key         | Action                         |
| ----------- | ------------------------------ |
| `q` / `Q`   | Quit application               |
| `c`         | Open connection dialog         |
| `Tab`       | Switch focus between panels    |
| `↑` / `↓`   | Navigate file list             |
| `Enter`     | Open directory / Download file |
| `Backspace` | Go to parent directory         |
| `r`         | Refresh file listing           |
| `u`         | Upload file                    |
| `Esc`       | Close dialog / Cancel          |

## 📁 Project Structure

```
rust-ftp-tui/
├── src/
│   ├── main.rs      # Entry point, terminal setup
│   ├── app.rs       # Application state, event handling
│   ├── ftp.rs       # FTP operations, connection management
│   ├── ui.rs        # TUI rendering, widgets
│   ├── config.rs    # Configuration management
│   └── lib.rs       # Library exports
├── examples/
│   └── test_ftp.rs  # FTP functionality test
├── Cargo.toml       # Dependencies
├── LICENSE          # MIT License
└── README.md        # This file
```

## ⚙️ Configuration

Configuration file is stored at `~/.config/rust-ftp-tui/config.json`:

```json
{
  "servers": [
    {
      "name": "My FTP Server",
      "host": "ftp.example.com",
      "port": 21,
      "username": "user",
      "use_tls": false
    }
  ],
  "default_local_path": "~/Downloads",
  "theme": {
    "primary_color": "blue",
    "accent_color": "yellow"
  }
}
```

## 🔧 Dependencies

| Crate                                                  | Purpose               |
| ------------------------------------------------------ | --------------------- |
| [tokio](https://tokio.rs/)                             | Async runtime         |
| [ratatui](https://ratatui.rs/)                         | TUI framework         |
| [suppaftp](https://github.com/veeso/suppaftp)          | FTP client library    |
| [crossterm](https://github.com/crossterm-rs/crossterm) | Terminal manipulation |
| [clap](https://clap.rs/)                               | CLI argument parsing  |
| [serde](https://serde.rs/)                             | Serialization         |
| [tracing](https://tracing.rs/)                         | Logging               |

## 🗺️ Roadmap

- [x] Basic FTP connection and authentication
- [x] File listing with directory navigation
- [x] File download functionality
- [x] Dual-panel TUI layout
- [x] Connection dialog
- [x] Configuration system
- [ ] Progress bars for file transfers
- [ ] Full local file system browser
- [ ] File search functionality
- [ ] FTPS (FTP over TLS) support
- [ ] SFTP support
- [ ] Bookmark favorite directories
- [ ] Transfer queue with multiple files
- [ ] Drag and drop support (terminal permitting)

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Ratatui](https://ratatui.rs/) for the amazing TUI framework
- [SuppaFTP](https://github.com/veeso/suppaftp) for the robust FTP library
- The Rust community for excellent tooling and documentation

## 📧 Contact

Project Link: [https://github.com/axpnet/rust-ftp-tui](https://github.com/axpnet/rust-ftp-tui)

---

Made with ❤️ and 🦀 Rust
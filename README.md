# PhantomFTP - CyberPunk TUI FTP Client

<div align="center">

<img src="icons/PhantomFTP_logo_color.svg" alt="PhantomFTP Logo" width="180">

### 👻 The Ghost in Your Terminal

![Rust](https://img.shields.io/badge/Rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![Terminal](https://img.shields.io/badge/Terminal-%23000000.svg?style=for-the-badge&logo=gnu-bash&logoColor=white)
![License](https://img.shields.io/github/license/axpnet/PhantomFTP?style=for-the-badge)
![Version](https://img.shields.io/badge/version-1.0.0-00aacc?style=for-the-badge)

```
    ██████╗ ██╗  ██╗ █████╗ ███╗   ██╗████████╗ ██████╗ ███╗   ███╗
    ██╔══██╗██║  ██║██╔══██╗████╗  ██║╚══██╔══╝██╔═══██╗████╗ ████║
    ██████╔╝███████║███████║██╔██╗ ██║   ██║   ██║   ██║██╔████╔██║
    ██╔═══╝ ██╔══██║██╔══██║██║╚██╗██║   ██║   ██║   ██║██║╚██╔╝██║
    ██║     ██║  ██║██║  ██║██║ ╚████║   ██║   ╚██████╔╝██║ ╚═╝ ██║
    ╚═╝     ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═══╝   ╚═╝    ╚═════╝ ╚═╝     ╚═╝
                    ███████╗████████╗██████╗ 
                    ██╔════╝╚══██╔══╝██╔══██╗
                    █████╗     ██║   ██████╔╝
                    ██╔══╝     ██║   ██╔═══╝ 
                    ██║        ██║   ██║     
                    ╚═╝        ╚═╝   ╚═╝     
```

**PhantomFTP** is a modern, asynchronous Terminal User Interface (TUI) FTP client built in Rust.  
*A futuristic, cyberpunk-inspired FTP client that flies like a phantom.*

</div>

## 🌟 Features

- **Multi-Protocol Support**:
  - ✅ FTP (Classic File Transfer Protocol)
  - ✅ FTPS (FTP over TLS/SSL for secure connections)
  - 🔜 SFTP (SSH File Transfer Protocol - coming in v1.1)
- **Dual-Panel Interface**: Local and remote file browsing side-by-side
- **Asynchronous Operations**: Non-blocking file transfers with tokio runtime
- **Real Progress Tracking**: Visual progress bars for downloads and uploads
- **Secure Authentication**: Username/password authentication
- **Intuitive TUI**: Terminal-based interface with keyboard navigation
- **Connection Management**: Save and reuse server connection settings
- **Transfer Resilience**: Automatic retry mechanism (up to 3 attempts)
- **Transfer Cancellation**: Cancel ongoing transfers with Ctrl+C
- **File Preview**: Preview remote files with the 'p' key

## 🚀 Installation

### From Source

Requirements:
- Rust 1.75 or higher
- Git

```bash
# Clone the repository
git clone https://github.com/axpnet/PhantomFTP.git
cd PhantomFTP

# Build the project
cargo build --release

# Run the application
./target/release/PhantomFTP
```

### Debian Package (Linux)

Download the `.deb` package from [Releases](https://github.com/axpnet/PhantomFTP/releases) and install:

```bash
sudo dpkg -i phantomftp_*.deb
```

## 🎮 Usage

After launching PhantomFTP, use the following keybindings:

### Navigation
- `Tab` - Switch between local and remote panels
- `↑/↓ Arrow Keys` - Navigate files in the selected panel
- `Enter` - Enter selected directory
- `Backspace` - Go to parent directory
- `Space` - Select/deselect files for batch operations

### Transfers
- `Enter` - Download (remote panel) or upload (local panel) selected file
- `p` - Preview remote file content
- `Ctrl+C` - Cancel ongoing transfer

### Connection
- `c` - Open connection dialog
- `h` - Show help dialog
- `q`/`Esc` - Quit the application

### Connection Dialog
In the connection dialog:
- `Tab` - Move between input fields
- `↑/↓` - Cycle between protocol options (FTP/FTPS/SFTP)
- `Enter` - Connect to the server
- `Esc` - Cancel and close dialog

## 🛠 Technical Architecture

PhantomFTP is built with modern Rust technologies:

- **[suppaftp](https://github.com/veeso/suppaftp)** - For FTP/FTPS protocol support
- **[russh](https://github.com/warp-tech/russh)** - For SSH/SFTP protocol support
- **[ratatui](https://github.com/ratatui-org/ratatui)** - For terminal user interface
- **[tokio](https://github.com/tokio-rs/tokio)** - For asynchronous runtime
- **[serde](https://github.com/serde-rs/serde)** - For configuration serialization

The application follows a modular architecture with clear separation of concerns:
- `src/main.rs` - Entry point
- `src/app.rs` - Application state and business logic
- `src/ui.rs` - Terminal user interface rendering
- `src/ftp.rs` - FTP/FTPS/SFTP protocol implementations
- `src/config.rs` - Configuration management

## ⚙️ Configuration

Configuration file is stored at `~/.config/PhantomFTP/config.json`:

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
- [x] Progress bars for file transfers
- [x] Full local file system browser
- [ ] File search functionality
- [ ] FTPS (FTP over TLS) support
- [ ] SFTP support
- [ ] Bookmark favorite directories
- [ ] Transfer queue with multiple files
- [ ] Drag and drop support (terminal permitting)

## 🤝 Contributors

Special thanks to these AI assistants who contributed to the development of PhantomFTP:

- **Qwen** - Primary development and architecture
- **Claude Opus 4.5** - Corrective implementation and debugging
- **Gemini 3 Pro** - Initial planning and design concepts
- **Grok** - Quick suggestions and optimizations

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 📞 Support

For issues, questions, or contributions, please [open an issue](https://github.com/axpnet/PhantomFTP/issues) on GitHub.
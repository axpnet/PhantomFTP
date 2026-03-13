# PhantomFTP - CyberPunk TUI FTP Client

<div align="center">

<img src="icons/PhantomFTP_logo_color.svg" alt="PhantomFTP Logo" width="180">

### The Ghost in Your Terminal

![Rust](https://img.shields.io/badge/Rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![Terminal](https://img.shields.io/badge/Terminal-%23000000.svg?style=for-the-badge&logo=gnu-bash&logoColor=white)
![License](https://img.shields.io/github/license/axpnet/PhantomFTP?style=for-the-badge)
![Version](https://img.shields.io/badge/version-2.0.0-00aacc?style=for-the-badge)

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

## Features

- **FTP & FTPS**: Plain FTP and Explicit TLS (FTPS) connections via [suppaftp](https://github.com/veeso/suppaftp)
- **Dual-Panel Interface**: Local and remote file browsing side-by-side
- **Asynchronous Transfers**: Non-blocking streaming uploads and downloads with [tokio](https://tokio.rs)
- **Real Progress Tracking**: Visual progress bars with percentage and byte count
- **Transfer Cancellation**: Cancel active transfers with Ctrl+C
- **Transfer Retry**: Automatic retry mechanism (up to 3 attempts)
- **File Preview**: Preview remote text files with the `p` key
- **File Operations**: Delete, rename, and create directories on the remote server
- **Connection Management**: Save and reuse server connection settings (JSON config)
- **CLI Arguments**: Connect directly from the command line (`--server`, `--username`, `--password`)
- **AeroFTP Import**: Import saved FTP/FTPS servers from [AeroFTP](https://github.com/AXP-OS/AeroFTP) exports
- **CyberPunk Aesthetics**: Neon-inspired terminal color scheme

## Installation

### From Source

Requirements:
- Rust 1.75 or higher
- Git

```bash
git clone https://github.com/axpnet/PhantomFTP.git
cd PhantomFTP
cargo build --release
./target/release/phantomftp
```

### Debian Package (Linux)

Download the `.deb` package from [Releases](https://github.com/axpnet/PhantomFTP/releases):

```bash
sudo dpkg -i phantomftp_*.deb
phantomftp
```

### Windows

Download `phantomftp-windows-amd64.exe` from [Releases](https://github.com/axpnet/PhantomFTP/releases).

## Usage

```bash
# Interactive mode
phantomftp

# Connect directly
phantomftp --server ftp.example.com --username myuser

# Import AeroFTP servers
phantomftp --import ~/exported-servers.json
```

### Keybindings

| Key | Action |
|-----|--------|
| `Tab` | Switch between local and remote panels |
| `Arrow Keys` | Navigate files |
| `Enter` | Enter directory / Download (remote) / Upload (local) |
| `Backspace` | Go to parent directory |
| `Space` | Select/deselect files |
| `c` | Open connection dialog |
| `p` | Preview remote file |
| `h` | Show help |
| `Ctrl+C` | Cancel active transfer (or quit if idle) |
| `q` / `Esc` | Quit |

### Connection Dialog

| Key | Action |
|-----|--------|
| `Tab` | Move between fields |
| `Up/Down` | Cycle protocol (FTP / FTPS) |
| `Enter` | Connect |
| `Esc` | Cancel |

## Configuration

Configuration is stored at `~/.config/phantomftp/config.json`:

```json
{
  "servers": [
    {
      "name": "My FTP Server",
      "host": "ftp.example.com",
      "port": 21,
      "username": "user",
      "protocol": "Ftp",
      "passive_mode": true,
      "timeout": 30
    }
  ],
  "default_local_path": "~/Downloads",
  "theme": {
    "primary_color": "blue",
    "accent_color": "yellow"
  }
}
```

Passwords are never saved to disk (`#[serde(skip)]`).

## AeroFTP Server Import

PhantomFTP can import saved servers from [AeroFTP](https://github.com/AXP-OS/AeroFTP) exports:

```bash
# Export servers from AeroFTP (Settings > Servers > Export)
# Then import into PhantomFTP:
phantomftp --import ~/aeroftp-servers.json
```

Only FTP and FTPS servers are imported — other protocols (SFTP, WebDAV, S3, etc.) are skipped.

## Architecture

```
src/
├── main.rs     Entry point and event loop
├── lib.rs      Public API exports
├── app.rs      Application state and business logic
├── ui.rs       Terminal UI rendering (ratatui)
├── ftp.rs      FTP/FTPS engine (suppaftp 8)
├── config.rs   Configuration management (serde)
├── banner.rs   ASCII art banner
└── import.rs   AeroFTP server import
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| [tokio](https://tokio.rs/) | Async runtime |
| [ratatui](https://ratatui.rs/) | TUI framework |
| [suppaftp](https://github.com/veeso/suppaftp) 8 | FTP/FTPS client (async, native-tls) |
| [crossterm](https://github.com/crossterm-rs/crossterm) | Terminal manipulation |
| [clap](https://clap.rs/) | CLI argument parsing |
| [serde](https://serde.rs/) | Serialization |
| [tracing](https://tracing.rs/) | Logging |

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Author

**axpdev** - [GitHub](https://github.com/axpnet)

## Support

For issues, questions, or contributions, please [open an issue](https://github.com/axpnet/PhantomFTP/issues) on GitHub.

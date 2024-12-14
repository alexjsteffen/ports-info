# PortsInfo

Simple utility to show information about all listening ports on Linux systems
![Screenshot From 2024-12-14 02-26-31](https://github.com/user-attachments/assets/0a9b1a36-c4ea-4b38-8229-30e59829f8f4)

This is a GTK4/libadwaita application that lists ports that are listening for connections on your Linux system. It displays all servers running on your system which can accept incoming connections.

The app uses netstat and ss commands under the hood. It requires administrative privileges to display additional info (process/command/PID). If you don't have root access, it falls back to a limited mode, displaying less information.

## Features
- List all TCP and UDP ports listening for connections
- Search through entries by port number or process name
- Ctrl+F shortcut for quick search
- Detailed process information when running with privileges:
  - Command line
  - CPU and memory usage
  - Start time
  - Process status
  - User

## Building from Source
Requirements:
- Rust 1.70 or newer
- GTK 4.0
- libadwaita 1.0
- pkg-config

On Ubuntu/Debian:
```bash
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev
cargo build --release
```

On Fedora:
```bash
sudo dnf install gtk4-devel libadwaita-devel gcc pkg-config
cargo build --release
```

## Running
```bash
cargo run
```

To get full process information:
```bash
pkexec ./target/release/ports-info
```

## Installation
Pre-built packages for various distributions are available in the [releases](https://github.com/mfat/ports-info/releases) section.

## License
GPL-3.0


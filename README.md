<p align="center">
  <img src="app-icon.png" alt="Divoom Monitor" width="128" height="128" />
</p>

<h1 align="center">Divoom Monitor</h1>

<p align="center">
  Desktop application for managing <a href="https://www.divoom.com/">Divoom</a> devices (Pixoo, Times Gate, etc.) over a local network.<br/>
  Built with <strong>Tauri 2 + Vue 3 + Rust</strong>.
</p>

<p align="center">
  <a href="https://github.com/Pisyukaev/divoom-monitor/releases"><img src="https://img.shields.io/github/v/release/Pisyukaev/divoom-monitor?style=flat-square" alt="Release" /></a>
  <a href="https://github.com/Pisyukaev/divoom-monitor/blob/main/LICENSE"><img src="https://img.shields.io/github/license/Pisyukaev/divoom-monitor?style=flat-square" alt="License" /></a>
</p>

<p align="center">
  <strong><a href="README_RU.md">Русская версия</a></strong>
</p>

---

## Features

### Device Discovery
Automatic detection of Divoom devices on the local network via the official Divoom API. Shows device type, IP/MAC address, signal strength, and connection status.

### Device Control
- Brightness adjustment
- Power on/off
- Mirror mode
- Temperature format toggle (°C / °F)
- 12/24-hour time format
- Device reboot

### Screen Editor (Times Gate)
- Visual editor for each screen
- Upload images from your computer or by URL
- Add text elements with customizable font, position, and size
- Push configuration to the device

### System Monitoring
- Real-time CPU, GPU, RAM, and disk usage
- CPU and GPU temperatures (Windows, via LibreHardwareMonitor)
- **PC Monitor** mode — automatically sends metrics to the device every 2 seconds

### App Settings
- Dark and light theme
- Launch at system startup
- Minimize to system tray on close
- In-app update check and installation
- English and Russian language support

---

## Tech Stack

| Layer | Technologies |
|-------|-------------|
| Backend | Rust, Tauri 2 |
| Frontend | Vue 3, TypeScript, Vite |
| UI | Element Plus |
| System Metrics | .NET 6 sidecar (LibreHardwareMonitor) |
| CI/CD | GitHub Actions, Tauri Action |

---

## Installation

Download the latest release from the [Releases](https://github.com/Pisyukaev/divoom-monitor/releases) page and run the installer.

The app supports auto-updates — when a new version is available, you will be notified directly in the settings.

---

## Building from Source

### Prerequisites

- [Node.js](https://nodejs.org/) (LTS)
- [pnpm](https://pnpm.io/)
- [Rust](https://www.rust-lang.org/tools/install)
- [.NET 6 SDK](https://dotnet.microsoft.com/download/dotnet/6.0) (for building the sidecar on Windows)

### Steps

```bash
# Clone the repository
git clone https://github.com/Pisyukaev/divoom-monitor.git
cd divoom-monitor

# Install dependencies
pnpm install

# Build the temperature monitoring sidecar (Windows)
pnpm build:sidecar

# Run in development mode
pnpm start

# Create a production build
pnpm tauri build
```

### Environment Variables

Copy `.env.example` to `.env` and adjust if needed:

| Variable | Description | Default |
|----------|-------------|---------|
| `LHM_SIDECAR_PATH` | Path to the HardwareMonitorCli executable | `sidecar/HardwareMonitorCli.exe` |

---

## License

MIT

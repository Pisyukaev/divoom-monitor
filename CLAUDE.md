# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Divoom Monitor is a Tauri 2 desktop app (Rust backend + Vue 3/TypeScript/Element Plus frontend) for controlling Divoom devices over the local network: device settings, per-screen image/text editing, and a "PC Monitor" mode that streams CPU/GPU/RAM/disk metrics to the device. On Windows, temperatures come from a separate .NET 6 sidecar built on LibreHardwareMonitor. The Times Gate (5 × 128×128 LCDs) is the most fully supported device; most screen-editing code assumes its layout.

## Commands

Package manager is **pnpm**. There are no tests and no linter configured; type-checking is the main correctness gate.

```bash
pnpm install
pnpm build:sidecar      # dotnet publish -> src-tauri/sidecar/ (Windows, needs .NET 6 SDK)
pnpm start              # tauri dev (runs `pnpm dev` = Vite on :1420, strictPort)
pnpm start:linux        # same, with WEBKIT_DISABLE_COMPOSITING_MODE=1
pnpm build              # vue-tsc --noEmit && vite build  (frontend type-check + bundle)
pnpm tauri build        # full installer; beforeBuildCommand runs build:sidecar + build
cargo check --manifest-path src-tauri/Cargo.toml   # fast Rust check
```

UI-only work doesn't need Rust: `pnpm dev` serves the frontend at http://localhost:1420, and outside Tauri `src/dev/mockTauri.ts` mocks every `invoke` (fake devices, settings, metrics). The mock is loaded only when `import.meta.env.DEV` and is not part of production builds. When you add a Tauri command, add a case to that mock too.

`pnpm tauri build` always rebuilds the sidecar, so it needs the .NET SDK even if you only changed frontend code. Without the sidecar the app still runs; it falls back to `sysinfo`/WMI/NVML and temperatures may be missing.

Releases: pushing to the `release` branch (or manual dispatch) runs `.github/workflows/release.yml` on `windows-latest`, which creates a **draft** GitHub release via tauri-action, signed with `TAURI_SIGNING_PRIVATE_KEY`. The version must be bumped in three places that are kept in sync: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`. The in-app updater reads `releases/latest/download/latest.json`.

## Architecture

### Three processes

1. **Vue webview** (`src/`) holds UI and nearly all persisted state, in `localStorage`.
2. **Rust core** (`src-tauri/src/`) exposes `#[tauri::command]`s and does all network I/O to devices and Divoom cloud, plus image processing.
3. **HardwareMonitorCli** (`hardware-monitor-cli/Program.cs`) is a .NET Worker that serves sensor JSON at `http://localhost:8765/` (and `/shutdown`). In production the NSIS hooks (`src-tauri/nsis/hooks.nsh`) register it as the auto-start Windows service `HardwareMonitorCli` (installer is `perMachine` because of this). In dev, `system_metrics::start_sidecar_service` probes port 8765, tries `sc start`, then spawns the exe directly from `LHM_SIDECAR_PATH` (via `.env`, loaded by `dotenvy`) or `<exe_dir>/sidecar/`.

### Talking to devices

- `divoom_api.rs` is the single transport: every device command is `POST http://<ip>/post` with JSON `{"Command": "...", ...}` (default 500 ms timeout; image upload uses 1 s).
- **Discovery is cloud-based, not LAN scanning**: `scan_devices` calls `https://app.divoom-gz.com/Device/ReturnSameLANDevice`, and `Hardware` codes are mapped to model names there (400 = Times Gate, 401–408 = Pixoo/Ditoo variants). A device the Divoom cloud does not report will not appear.
- `get_lcd_info` also needs the cloud (`Channel/Get5LcdInfoV2`) to get `DeviceId` and `LcdIndependence`, which are required to activate PC Monitor (`Channel/SetClockSelectId`, `ClockId: 625`) before metrics are pushed with `Device/UpdatePCParaInfo`.
- Images are resized to exactly 128×128, JPEG-encoded, base64'd, and sent as `Draw/SendHttpGif` with an `LCDArray` one-hot over 5 screens. `PicID` comes from a process-global counter starting at 1000.
- Many setter commands in `device_commands.rs` swallow errors and return `()`; the frontend cannot tell whether they succeeded.

### Adding a Tauri command

A new command must be wired in **all** of these, or it fails at runtime with no compile error:
- the function in the appropriate `src-tauri/src/*_commands.rs` / module,
- `tauri::generate_handler![...]` in `src-tauri/src/lib.rs`,
- the `commands` tuple in `src/constants.ts` if it goes through the typed `invokeCommand` helper (`src/api/common.ts`); otherwise a wrapper in `src/api/*.ts`,
- `src-tauri/capabilities/default.json` if it needs a plugin permission.

Tauri converts Rust `snake_case` args to camelCase on the JS side (`ip_address` becomes `ipAddress`), but fields **inside** a serde struct argument keep their Rust names (e.g. `textConfig.text_width` in `useAutoSendConfig.ts`). Return structs are also snake_case in TS (`src/types/*.ts` mirror `models.rs`).

### Frontend state and startup

- Layout: `App.vue` is a shell with a persistent `AppSidebar` (device list and settings) and a `<RouterView>`. The discovered device list is module-level shared state in `composables/useDevices.ts` and is scanned once per session. Design tokens and shared classes (`.app-page`, `.app-card`, ...) live in `src/styles/theme.css`, which overrides Element Plus `--el-*` variables for light and `html.dark`.
- Routes: `/` home (device cards), `/settings` app settings, `/device/:id/{common,display,system}` (control, Times Gate screens, PC Monitor). The `display` tab is hidden for devices other than Times Gate. `:id` is `encodeURIComponent(ip || mac || name)` (`getDeviceRouteId` in `useDevices.ts`, duplicated in `useAutoSendConfig.ts`); `utils/device.ts#parseDeviceIp` gets the IP back.
- localStorage keys: `screen_configs_<deviceId>` (Times Gate screen editor, `ScreenManager.vue`), `pc_monitor_<ip>` (PC Monitor settings), plus theme, `app_locale`, `sidebarWidth`.
- On app mount, `App.vue` calls `startPcMonitorForAllDevices()` (re-activates ClockId 625 with retries, then pushes metrics every 2 s from a `setInterval` in the webview) and `sendConfigsToAllDevices()` (re-scans and re-sends saved screen configs). Both loops live in the webview, so they only run while the webview is alive (including when hidden to tray).
- The only setting persisted on the Rust side is `close_to_tray` (`app_settings.rs`, written to `<app_data_dir>/settings.json`), because the window-close handler needs it before JS runs. Autostart passes `--minimized`, which hides the window at launch.
- UI strings go through vue-i18n; add keys to both `src/i18n/locales/en.ts` and `ru.ts`. The tray menu labels in `lib.rs` are hard-coded Russian.

### Platform notes

- Windows-only code is behind `cfg(target_os = "windows")` (WMI, NVML, `CREATE_NO_WINDOW`, `sc`/`taskkill`). Keep non-Windows builds compiling when touching `system_metrics.rs`.
- `stop_sidecar_service` runs on quit, on close when not minimizing to tray, and from the panic hook.
- Code comments are a mix of English and Russian.

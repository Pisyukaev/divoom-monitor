# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Divoom Monitor is a Tauri 2 desktop app (Rust backend + Vue 3/TypeScript/Element Plus frontend) for controlling Divoom devices over the local network: device settings, per-screen image/text editing, and a "PC Monitor" mode that streams CPU/GPU/RAM/disk metrics to the device. On Windows, temperatures come from a separate .NET 6 sidecar built on LibreHardwareMonitor. The Times Gate (5 × 128×128 LCDs) is the most fully supported device; most screen-editing code assumes its layout.

## Commands

Package manager is **pnpm**. Rust unit tests cover the Divoom API layer (`pnpm test` = `cargo test`); there are no frontend tests and no linter, so type-checking is the other correctness gate.

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

- Official API docs: http://doc.divoom-gz.com/web/#/12 (section "TimeGate"). Payload builders and response parsers are in `divoom_protocol.rs` (pure, tested against the docs' examples); keep new commands there.
- `divoom_api.rs` is the single transport: every device command is `POST http://<ip>/post` with JSON `{"Command": "...", ...}` (default 500 ms timeout; image upload uses 1 s). A non-zero `error_code` from the device (number or string), or `ReturnCode` from the cloud, is returned as `Err`. Exception: newer Times Gate firmware answers `"DeviceToken is err"` to every request without a valid `LocalToken` (an undocumented numeric body field, shown in the Divoom app under the device's settings). Verified on a device: writes such as images and brightness are still applied, so this reply passes as success; reads come back empty, so `get_device_info` checks `is_token_error` and fails with `LOCAL_TOKEN_REQUIRED: ...`, and the control screen then keeps the controls with unknown values.
- Screen text is not sent with the device's text commands. Tested on a real Times Gate: `Draw/SendHttpText` returns `error_code 0` but draws nothing; `Draw/SendHttpItemList` hangs the LCD on "loading..." unless `BackgroudGif` is an internet GIF URL (so no local image under the text), and its free-text items appear only now and then. Instead `utils/screenRender.ts` draws each screen (image with `object-fit: cover`, then texts clipped to their area) on a 128×128 canvas in the webview and sends the PNG with `upload_image_data` → `Draw/SendHttpGif`. URL images are fetched by Rust (`fetch_image_base64`) because a cross-origin image would taint the canvas. `ScreenPreview.vue` shows that same rendered picture, so the preview matches the device; fonts are Windows fonts (`FONT_OPTIONS`, ids 0–7 kept from older configs). The screen editor sends 400 ms after the last edit; startup re-sends only screens with an image or texts.
- `LocalToken` is per device. The frontend persists it (`utils/localToken.ts`) and pushes it to Rust with `set_local_token`; `divoom_api.rs` keeps it in memory by IP and adds it to every `send_command` for that IP. The Rust copy is lost on restart, so `App.vue` awaits `syncLocalTokens()` before starting any device loop. Tests run it against a `wiremock` server; cloud calls take a base URL for that reason (`*_at` / `*_from` functions).
- **Discovery is cloud-based, not LAN scanning**: `scan_devices` calls `https://app.divoom-gz.com/Device/ReturnSameLANDevice`, and `Hardware` codes are mapped to model names there (400 = Times Gate, 401–408 = Pixoo/Ditoo variants). A device the Divoom cloud does not report will not appear.
- `get_lcd_info` also needs the cloud (`Channel/Get5LcdInfoV2`, POST with JSON body) to get `DeviceId` and the selected `LcdIndependence` (`selectedIndependence` in `types/system.ts`), which are required to activate PC Monitor (`Channel/SetClockSelectId`, `ClockId: 625`) before metrics are pushed with `Device/UpdatePCParaInfo`.
- Images are resized to exactly 128×128, JPEG-encoded, base64'd, and sent as `Draw/SendHttpGif` with an `LcdArray` one-hot over 5 screens. `PicID` comes from a process-global counter starting at 1000.
- Many setter commands in `device_commands.rs` swallow errors and return `()`; the frontend cannot tell whether they succeeded.

### Adding a Tauri command

A new command must be wired in **all** of these, or it fails at runtime with no compile error:
- the function in the appropriate `src-tauri/src/*_commands.rs` / module,
- `tauri::generate_handler![...]` in `src-tauri/src/lib.rs`,
- the `commands` tuple in `src/constants.ts` if it goes through the typed `invokeCommand` helper (`src/api/common.ts`); otherwise a wrapper in `src/api/*.ts`,
- `src-tauri/capabilities/default.json` if it needs a plugin permission.

Tauri converts Rust `snake_case` args to camelCase on the JS side (`ip_address` becomes `ipAddress`), but fields **inside** a serde struct argument keep their Rust names (`text_width`, not `textWidth`). Return structs are also snake_case in TS (`src/types/*.ts` mirror `models.rs`).

### Frontend state and startup

- Layout: `App.vue` is a shell with a persistent `AppSidebar` (device list and settings) and a `<RouterView>`. The discovered device list is module-level shared state in `composables/useDevices.ts` and is scanned once per session. Design tokens and shared classes (`.app-page`, `.app-card`, ...) live in `src/styles/theme.css`, which overrides Element Plus `--el-*` variables for light and `html.dark`.
- Routes: `/` home (device cards), `/settings` app settings, `/device/:id/{common,display,system}` (control, Times Gate screens, PC Monitor). The `display` tab is hidden for devices other than Times Gate. `:id` is `encodeURIComponent(ip || mac || name)` (`getDeviceRouteId` in `useDevices.ts`, duplicated in `useAutoSendConfig.ts`); `utils/device.ts#parseDeviceIp` gets the IP back.
- localStorage keys: `screen_configs_<deviceId>` (Times Gate screen editor, `ScreenManager.vue`), `pc_monitor_<ip>` (PC Monitor settings), `local_token_<ip>` (device LocalToken), plus theme, `app_locale`, `sidebarWidth`.
- On app mount, `App.vue` calls `startPcMonitorForAllDevices()` (re-activates ClockId 625 with retries, then pushes metrics every 2 s from a `setInterval` in the webview) and `sendConfigsToAllDevices()` (re-scans and re-sends saved screen configs). Both loops live in the webview, so they only run while the webview is alive (including when hidden to tray).
- The only setting persisted on the Rust side is `close_to_tray` (`app_settings.rs`, written to `<app_data_dir>/settings.json`), because the window-close handler needs it before JS runs. Autostart passes `--minimized`, which hides the window at launch.
- UI strings go through vue-i18n; add keys to both `src/i18n/locales/en.ts` and `ru.ts`. The tray menu labels in `lib.rs` are hard-coded Russian.

### Platform notes

- Windows-only code is behind `cfg(target_os = "windows")` (WMI, NVML, `CREATE_NO_WINDOW`, `sc`/`taskkill`). Keep non-Windows builds compiling when touching `system_metrics.rs`.
- `stop_sidecar_service` runs on quit, on close when not minimizing to tray, and from the panic hook.
- Code comments are a mix of English and Russian.

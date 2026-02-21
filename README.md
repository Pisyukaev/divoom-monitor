# Tauri + Vue + TypeScript

This template should help get you started developing with Vue 3 and TypeScript in Vite. The template uses Vue 3 `<script setup>` SFCs, check out the [script setup docs](https://v3.vuejs.org/api/sfc-script-setup.html#sfc-script-setup) to learn more.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Hardware temperature sidecar (Windows)

To get CPU/GPU temperatures on Windows, build the .NET sidecar:

```bash
pnpm build:sidecar
```

This publishes `HardwareMonitorCli.exe` into `src-tauri/sidecar/`. The Tauri backend discovers it automatically.

To override the sidecar location, set `LHM_SIDECAR_PATH` in the `.env` file at the repo root.

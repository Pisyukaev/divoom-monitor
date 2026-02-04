# Tauri + Vue + TypeScript

This template should help get you started developing with Vue 3 and TypeScript in Vite. The template uses Vue 3 `<script setup>` SFCs, check out the [script setup docs](https://v3.vuejs.org/api/sfc-script-setup.html#sfc-script-setup) to learn more.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Hardware temperature sidecar (Windows)

To get CPU/GPU temperatures on Windows, build the optional .NET sidecar and point the app to it:

```bash
cd src-tauri/sidecar/HardwareMonitorCli
dotnet publish -c Release -r win-x64
```

Then set the environment variable before running Tauri:

```bash
set LHM_SIDECAR_PATH=src-tauri\sidecar\HardwareMonitorCli\bin\Release\net8.0\win-x64\publish\HardwareMonitorCli.exe
pnpm run start
```

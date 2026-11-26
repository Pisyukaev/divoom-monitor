/**
 * Dev-only Tauri IPC mock so the UI can be developed in a plain browser
 * (`pnpm dev`, http://localhost:1420) without the Rust backend or a device.
 * Loaded from main.ts only when `import.meta.env.DEV` and not inside Tauri.
 */
import { mockIPC, mockWindows } from '@tauri-apps/api/mocks';

const devices = [
  {
    name: 'Times Gate',
    mac_address: 'a8:03:2a:11:22:33',
    device_type: 'Times Gate',
    ip_address: '192.168.1.42',
    signal_strength: null,
    is_connected: true,
    device_id: 300012345,
  },
  {
    name: 'Pixoo 64',
    mac_address: 'a8:03:2a:44:55:66',
    device_type: 'Pixoo 64',
    ip_address: '192.168.1.57',
    signal_strength: null,
    is_connected: true,
    device_id: 300067890,
  },
];

const deviceSettings: Record<string, unknown> = {
  brightness: 70,
  rotation_flag: 0,
  date_format: 'dd.mm.yyyy',
  time24_flag: 1,
  temperature_mode: 0,
  mirror_flag: 0,
  light_switch: 1,
};

const lcdClocks = [625, 0, 12, 0, 0];
let closeToTray = true;
let autostart = false;

const jitter = (base: number, spread: number) =>
  Math.max(0, Math.min(100, base + (Math.random() - 0.5) * spread));

function systemMetrics() {
  const gb = 1024 ** 3;
  return {
    cpu_usage: jitter(34, 20),
    cpu_temperature: jitter(56, 6),
    gpu_usage: jitter(18, 14),
    gpu_temperature: jitter(47, 4),
    memory_total: 32 * gb,
    memory_used: 13.4 * gb + Math.random() * 0.5 * gb,
    disks: [
      { name: 'System', mount_point: 'C:\\', total_space: 1000 * gb, available_space: 380 * gb, used_space: 620 * gb, usage_percent: 62 },
      { name: 'Data', mount_point: 'D:\\', total_space: 2000 * gb, available_space: 1500 * gb, used_space: 500 * gb, usage_percent: 25 },
    ],
  };
}

const delay = (ms: number) => new Promise((r) => setTimeout(r, ms));

export function installTauriMock() {
  mockWindows('main');
  mockIPC(async (cmd, payload) => {
    const args = (payload ?? {}) as Record<string, unknown>;
    switch (cmd) {
      case 'scan_devices':
        await delay(600);
        return devices;
      case 'get_device_info':
        await delay(300);
        return { ...deviceSettings };
      case 'set_brightness':
        deviceSettings.brightness = args.value;
        return null;
      case 'set_switch_screen':
        deviceSettings.light_switch = args.value;
        return null;
      case 'set_temperature_mode':
        deviceSettings.temperature_mode = args.value;
        return null;
      case 'set_mirror_mode':
        deviceSettings.mirror_flag = args.value;
        return null;
      case 'set_24_hours_mode':
        deviceSettings.time24_flag = args.value;
        return null;
      case 'reboot_device':
      case 'upload_image_from_url':
      case 'upload_image_from_file':
      case 'set_screen_text':
      case 'send_pc_metrics':
        await delay(200);
        return null;
      case 'get_lcd_info':
        await delay(300);
        return {
          device_id: 300012345,
          channel_type: 1,
          lcd_independence: 1,
          independence_list: [
            { lcd_independence: 1, lcd_list: lcdClocks.map((id) => ({ lcd_clock_id: id })) },
          ],
        };
      case 'activate_pc_monitor':
        lcdClocks[args.lcdIndex as number] = 625;
        return null;
      case 'get_system_metrics':
        return systemMetrics();
      case 'get_close_to_tray':
        return closeToTray;
      case 'set_close_to_tray':
        closeToTray = Boolean(args.enabled);
        return null;
      case 'plugin:app|version':
        return '0.1.1-dev';
      case 'plugin:autostart|is_enabled':
        return autostart;
      case 'plugin:autostart|enable':
        autostart = true;
        return null;
      case 'plugin:autostart|disable':
        autostart = false;
        return null;
      case 'plugin:updater|check':
        await delay(500);
        return null;
      case 'plugin:dialog|open':
        return null;
      default:
        console.warn('[tauri-mock] unhandled command', cmd, args);
        return null;
    }
  });
  console.info('[tauri-mock] Tauri IPC is mocked (browser dev mode)');
}

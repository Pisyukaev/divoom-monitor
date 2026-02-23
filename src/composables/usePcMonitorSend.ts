import {
  getSystemMetrics,
  sendPcMetrics,
  getLcdInfo,
  activatePcMonitor,
} from '../api/system';
import type { SystemMetrics } from '../types/system';

const STORAGE_KEY_PREFIX = 'pc_monitor_';
const SEND_INTERVAL_MS = 2000;
const ACTIVATION_MAX_RETRIES = 3;
const ACTIVATION_RETRY_DELAY_MS = 3000;

export interface PcMonitorSettings {
  lcdIndex: number;
  enabled: boolean;
}

const activeLoops = new Map<string, number>();

export function buildDispData(m: SystemMetrics): string[] {
  const cpuUsage = `${Math.round(m.cpu_usage)}%`;
  const gpuUsage = '0%';
  const cpuTemp =
    m.cpu_temperature !== null ? `${Math.round(m.cpu_temperature)} C` : 'N/A';
  const gpuTemp =
    m.gpu_temperature !== null ? `${Math.round(m.gpu_temperature)} C` : 'N/A';
  const ramUsage = `${Math.round(m.memory_total > 0 ? (m.memory_used / m.memory_total) * 100 : 0)}%`;

  let hddUsage = '0%';
  if (m.disks.length > 0) {
    const maxPct = Math.max(...m.disks.map((d) => d.usage_percent));
    hddUsage = `${Math.round(maxPct)}%`;
  }

  return [cpuUsage, gpuUsage, cpuTemp, gpuTemp, ramUsage, hddUsage];
}

export function savePcMonitorSettings(
  deviceIp: string,
  settings: PcMonitorSettings
): void {
  localStorage.setItem(
    `${STORAGE_KEY_PREFIX}${deviceIp}`,
    JSON.stringify(settings)
  );
}

export function loadPcMonitorSettings(
  deviceIp: string
): PcMonitorSettings | null {
  const stored = localStorage.getItem(`${STORAGE_KEY_PREFIX}${deviceIp}`);
  if (!stored) return null;

  try {
    return JSON.parse(stored) as PcMonitorSettings;
  } catch {
    return null;
  }
}

function getAllSavedPcMonitorEntries(): {
  ip: string;
  settings: PcMonitorSettings;
}[] {
  const entries: { ip: string; settings: PcMonitorSettings }[] = [];

  for (let i = 0; i < localStorage.length; i++) {
    const key = localStorage.key(i);
    if (!key?.startsWith(STORAGE_KEY_PREFIX)) continue;

    const ip = key.slice(STORAGE_KEY_PREFIX.length);
    if (!ip) continue;

    try {
      const settings = JSON.parse(
        localStorage.getItem(key)!
      ) as PcMonitorSettings;
      entries.push({ ip, settings });
    } catch {
      // skip malformed entries
    }
  }

  return entries;
}

export function isPcMonitorRunning(deviceIp: string): boolean {
  return activeLoops.has(deviceIp);
}

export function startPcMonitorLoop(deviceIp: string, lcdIndex: number): void {
  if (activeLoops.has(deviceIp)) {
    stopPcMonitorLoop(deviceIp);
  }

  async function tick() {
    try {
      const metrics = await getSystemMetrics();
      const dispData = buildDispData(metrics);
      await sendPcMetrics(deviceIp, lcdIndex, dispData);
    } catch (err) {
      console.error(`[PC Monitor] Error sending metrics to ${deviceIp}:`, err);
    }
  }

  tick();
  const handle = window.setInterval(tick, SEND_INTERVAL_MS);
  activeLoops.set(deviceIp, handle);
}

export function stopPcMonitorLoop(deviceIp: string): void {
  const handle = activeLoops.get(deviceIp);
  if (handle !== undefined) {
    window.clearInterval(handle);
    activeLoops.delete(deviceIp);
  }
}

async function activateWithRetry(
  ip: string,
  lcdIndex: number
): Promise<boolean> {
  for (let attempt = 0; attempt <= ACTIVATION_MAX_RETRIES; attempt++) {
    try {
      console.log(
        `[PC Monitor] Activating PC monitor on ${ip}, screen ${lcdIndex} (attempt ${attempt + 1})`
      );
      const info = await getLcdInfo(ip);
      const independence = info.independence_list[0];
      if (independence) {
        await activatePcMonitor(
          ip,
          info.device_id,
          independence.lcd_independence,
          lcdIndex
        );
        console.log(
          `[PC Monitor] Activated ClockId 625 on ${ip}, screen ${lcdIndex}`
        );
        return true;
      }
    } catch (err) {
      console.error(
        `[PC Monitor] Activation attempt ${attempt + 1} failed for ${ip}:`,
        err
      );
    }

    if (attempt < ACTIVATION_MAX_RETRIES) {
      await new Promise((resolve) =>
        setTimeout(resolve, ACTIVATION_RETRY_DELAY_MS)
      );
    }
  }
  return false;
}

export async function startPcMonitorForAllDevices(): Promise<void> {
  const entries = getAllSavedPcMonitorEntries();

  const tasks = entries
    .filter(({ settings }) => settings.enabled)
    .map(async ({ ip, settings }) => {
      const activated = await activateWithRetry(ip, settings.lcdIndex);
      if (!activated) {
        console.error(
          `[PC Monitor] All activation attempts failed for ${ip}, skipping metrics loop`
        );
        return;
      }

      console.log(
        `[PC Monitor] Auto-starting metrics send to ${ip}, screen ${settings.lcdIndex}`
      );
      startPcMonitorLoop(ip, settings.lcdIndex);
    });

  await Promise.allSettled(tasks);
}

import {
  startDota2Server,
  stopDota2Server,
  getDota2Status,
  detectDota2Path,
  setupDota2GsiConfig,
} from '../api/dota2';
import { sendConfigsToDevice } from './useAutoSendConfig';
import type { ScreenConfigs } from '../types/screen';
import type { Dota2Settings, Dota2Status } from '../types/dota2';

const SETTINGS_KEY_PREFIX = 'dota2_';
const SAVED_STATE_KEY_PREFIX = 'dota2_saved_configs_';

export function saveDota2Settings(
  deviceIp: string,
  settings: Dota2Settings,
): void {
  localStorage.setItem(
    `${SETTINGS_KEY_PREFIX}${deviceIp}`,
    JSON.stringify(settings),
  );
}

export function loadDota2Settings(
  deviceIp: string,
): Dota2Settings | null {
  const stored = localStorage.getItem(`${SETTINGS_KEY_PREFIX}${deviceIp}`);
  if (!stored) return null;

  try {
    return JSON.parse(stored) as Dota2Settings;
  } catch {
    return null;
  }
}

function saveCurrentDisplayState(deviceIp: string, deviceId: string): void {
  const configKey = `screen_configs_${deviceId}`;
  const current = localStorage.getItem(configKey);
  if (current) {
    localStorage.setItem(`${SAVED_STATE_KEY_PREFIX}${deviceIp}`, current);
  }
}

async function restoreDisplayState(
  deviceIp: string,
): Promise<void> {
  const savedKey = `${SAVED_STATE_KEY_PREFIX}${deviceIp}`;
  const saved = localStorage.getItem(savedKey);

  if (saved) {
    try {
      const configs: ScreenConfigs = JSON.parse(saved);
      await sendConfigsToDevice(deviceIp, configs);
      localStorage.removeItem(savedKey);
    } catch (error) {
      console.error('[Dota2] Failed to restore display state:', error);
    }
  }
}

export async function startDota2Integration(
  deviceIp: string,
  deviceId: string,
  port: number,
): Promise<void> {
  saveCurrentDisplayState(deviceIp, deviceId);
  await startDota2Server(deviceIp, port);
}

export async function stopDota2Integration(
  deviceIp: string,
): Promise<void> {
  await stopDota2Server();
  await restoreDisplayState(deviceIp);
}

export async function fetchDota2Status(): Promise<Dota2Status> {
  return getDota2Status();
}

export async function autoDetectDota2Path(): Promise<string | null> {
  return detectDota2Path();
}

export async function configureDota2Gsi(
  dotaPath: string,
  port: number,
): Promise<string> {
  return setupDota2GsiConfig(dotaPath, port);
}

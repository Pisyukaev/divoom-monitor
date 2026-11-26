import { ref } from 'vue';

import { scanDevices } from '../api/common';
import type { DivoomDevice } from '../types/device';

const MAX_AUTO_RETRIES = 3;
const RETRY_DELAY_MS = 3000;

// Module-level state: the device list is shared by the sidebar, the home page and device pages.
const devices = ref<DivoomDevice[]>([]);
const isScanning = ref(false);
const hasScanned = ref(false);
let initialScan: Promise<void> | null = null;

export function getDeviceRouteId(device: DivoomDevice): string {
  return encodeURIComponent(device.ip_address || device.mac_address || device.name);
}

async function scan() {
  isScanning.value = true;
  try {
    devices.value = await scanDevices();
  } finally {
    isScanning.value = false;
    hasScanned.value = true;
  }
}

async function scanWithRetry() {
  for (let attempt = 0; attempt <= MAX_AUTO_RETRIES; attempt++) {
    await scan();
    if (devices.value.length > 0) return;
    if (attempt < MAX_AUTO_RETRIES) {
      await new Promise((resolve) => setTimeout(resolve, RETRY_DELAY_MS));
    }
  }
}

export function useDevices() {
  /** Runs the initial discovery (with retries) once per app session. */
  function ensureScanned() {
    if (!initialScan) {
      initialScan = scanWithRetry();
    }
    return initialScan;
  }

  /** Reactive when called inside a computed/template. */
  function findByRouteId(routeId: string): DivoomDevice | null {
    return devices.value.find((d) => getDeviceRouteId(d) === routeId) ?? null;
  }

  return {
    devices,
    isScanning,
    hasScanned,
    scan,
    ensureScanned,
    findByRouteId,
  };
}

export function isTimesGate(device: DivoomDevice | null): boolean {
  return device?.device_type === 'Times Gate';
}

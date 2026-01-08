import { invoke } from '@tauri-apps/api/core';

import { DivoomDevice } from '../types/device';

export const scanDevices = async () => {
  let foundDevices: DivoomDevice[] = [];
  try {
    foundDevices = await invoke<DivoomDevice[]>('scan_devices');
    return foundDevices;
  } catch (err) {
    console.error('Error scanning devices:', err);
  } finally {
    return foundDevices;
  }
};

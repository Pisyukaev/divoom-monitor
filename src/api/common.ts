import { invoke } from '@tauri-apps/api/core';

import { DivoomDevice } from '../types/device';

export const scanDevices = async (): Promise<DivoomDevice[]> => {
  try {
    return await invoke<DivoomDevice[]>('scan_devices');
  } catch (err) {
    console.error('Error scanning devices:', err);
    return [];
  }
};

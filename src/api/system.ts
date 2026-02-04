import { invoke } from '@tauri-apps/api/core';

import type { SystemMetrics } from '../types/system';

export const getSystemMetrics = async (): Promise<SystemMetrics> => {
  return invoke<SystemMetrics>('get_system_metrics');
};

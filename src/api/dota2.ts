import { invoke } from '@tauri-apps/api/core';

import type { Dota2Status } from '../types/dota2';

export const startDota2Server = async (
  deviceIp: string,
  port: number,
): Promise<void> => {
  return invoke('start_dota2_server', { deviceIp, port });
};

export const stopDota2Server = async (): Promise<void> => {
  return invoke('stop_dota2_server');
};

export const getDota2Status = async (): Promise<Dota2Status> => {
  return invoke<Dota2Status>('get_dota2_status');
};

export const detectDota2Path = async (): Promise<string | null> => {
  return invoke<string | null>('detect_dota2_path');
};

export const setupDota2GsiConfig = async (
  dotaPath: string,
  port: number,
): Promise<string> => {
  return invoke<string>('setup_dota2_gsi_config', { dotaPath, port });
};

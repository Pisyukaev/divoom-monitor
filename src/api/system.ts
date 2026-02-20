import { invoke } from '@tauri-apps/api/core';

import type { LcdInfoResponse, SystemMetrics } from '../types/system';

export const getSystemMetrics = async (): Promise<SystemMetrics> => {
  return invoke<SystemMetrics>('get_system_metrics');
};

export const getLcdInfo = async (ipAddress: string): Promise<LcdInfoResponse> => {
  return invoke<LcdInfoResponse>('get_lcd_info', { ipAddress });
};

export const activatePcMonitor = async (
  ipAddress: string,
  deviceId: number,
  lcdIndependence: number,
  lcdIndex: number,
): Promise<void> => {
  return invoke('activate_pc_monitor', {
    ipAddress,
    deviceId,
    lcdIndependence,
    lcdIndex,
  });
};

export const sendPcMetrics = async (
  ipAddress: string,
  lcdIndex: number,
  dispData: string[],
): Promise<void> => {
  return invoke('send_pc_metrics', { ipAddress, lcdIndex, dispData });
};

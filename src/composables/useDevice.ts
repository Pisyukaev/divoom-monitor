import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { DivoomDevice, DeviceSettings } from '../types/device';

export function useDevice() {
  const device = ref<DivoomDevice | null>(null);
  const settings = ref<DeviceSettings | null>(null);
  const isLoadingSettings = ref(false);
  const settingsError = ref<string | null>(null);

  function getDeviceId(device: DivoomDevice): string {
    return encodeURIComponent(
      device.ip_address || device.mac_address || device.name
    );
  }

  function decodeDeviceId(id: string): string {
    return decodeURIComponent(id);
  }

  async function fetchDeviceSettings(
    id: string
  ): Promise<DeviceSettings | null> {
    try {
      const decodedId = decodeDeviceId(id);
      // Try to get device info by IP address
      if (decodedId.match(/^\d+\.\d+\.\d+\.\d+$/)) {
        const deviceInfo = await invoke<DeviceSettings>('get_device_info', {
          ipAddress: decodedId,
        });
        settings.value = deviceInfo;
        return deviceInfo;
      }
      return null;
    } catch (error) {
      console.error('Error fetching device:', error);
      return null;
    }
  }

  return {
    device,
    settings,
    isLoadingSettings,
    settingsError,
    getDeviceId,
    decodeDeviceId,
    fetchDeviceSettings,
  };
}

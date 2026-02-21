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
    isLoadingSettings.value = true;
    settingsError.value = null;
    try {
      const decodedId = decodeDeviceId(id);
      // Try to get device info by IP address
      if (/^(\d{1,3}\.){3}\d{1,3}$/.test(decodedId)) {
        const deviceInfo = await invoke<DeviceSettings>('get_device_info', {
          ipAddress: decodedId,
        });
        settings.value = deviceInfo;
        isLoadingSettings.value = false;
        return deviceInfo;
      }
      isLoadingSettings.value = false;
      settingsError.value = 'Неверный формат IP адреса';
      return null;
    } catch (error) {
      console.error('Error fetching device:', error);
      isLoadingSettings.value = false;
      settingsError.value = error instanceof Error ? error.message : 'Ошибка загрузки настроек устройства';
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

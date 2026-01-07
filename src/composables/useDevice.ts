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

  async function fetchDeviceById(id: string): Promise<DivoomDevice | null> {
    try {
      const decodedId = decodeDeviceId(id);
      // Try to get device info by IP address
      if (decodedId.match(/^\d+\.\d+\.\d+\.\d+$/)) {
        const deviceInfo = await invoke<DivoomDevice>('get_device_info', {
          ipAddress: decodedId,
        });
        return deviceInfo;
      }
      return null;
    } catch (error) {
      console.error('Error fetching device:', error);
      return null;
    }
  }

  async function fetchDeviceSettings(ipAddress: string): Promise<void> {
    if (!ipAddress) {
      settingsError.value = 'IP адрес устройства не указан';
      return;
    }

    isLoadingSettings.value = true;
    settingsError.value = null;

    try {
      const deviceSettings = await invoke<DeviceSettings>(
        'get_device_settings',
        {
          ipAddress,
        }
      );
      settings.value = deviceSettings;
    } catch (error) {
      settingsError.value =
        error instanceof Error
          ? error.message
          : 'Не удалось загрузить настройки';
      console.error('Error fetching device settings:', error);
      settings.value = null;
    } finally {
      isLoadingSettings.value = false;
    }
  }

  return {
    device,
    settings,
    isLoadingSettings,
    settingsError,
    getDeviceId,
    decodeDeviceId,
    fetchDeviceById,
    fetchDeviceSettings,
  };
}

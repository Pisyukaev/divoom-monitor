import { ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import type { DivoomDevice, DeviceSettings } from '../types/device';
import { syncLocalTokens, TOKEN_REQUIRED_ERROR } from '../utils/localToken';

/** Settings with every value unknown; controls still work (writes need no token). */
const UNKNOWN_SETTINGS: DeviceSettings = {
  brightness: null,
  rotation_flag: null,
  date_format: null,
  time24_flag: null,
  temperature_mode: null,
  mirror_flag: null,
  light_switch: null,
};

export function useDevice() {
  const { t } = useI18n();
  const device = ref<DivoomDevice | null>(null);
  const settings = ref<DeviceSettings | null>(null);
  const isLoadingSettings = ref(false);
  const settingsError = ref<string | null>(null);
  /** The device refused to return settings without a (valid) LocalToken. */
  const tokenRequired = ref(false);

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
    tokenRequired.value = false;
    try {
      const decodedId = decodeDeviceId(id);
      if (/^(\d{1,3}\.){3}\d{1,3}$/.test(decodedId)) {
        await syncLocalTokens();
        const deviceInfo = await invoke<DeviceSettings>('get_device_info', {
          ipAddress: decodedId,
        });
        settings.value = deviceInfo;
        isLoadingSettings.value = false;
        return deviceInfo;
      }
      isLoadingSettings.value = false;
      settingsError.value = t('commonSettings.invalidIp');
      return null;
    } catch (error) {
      isLoadingSettings.value = false;
      if (String(error).startsWith(TOKEN_REQUIRED_ERROR)) {
        console.warn('Device settings need a LocalToken:', error);
        tokenRequired.value = true;
        settings.value = { ...UNKNOWN_SETTINGS };
        return null;
      }
      console.error('Error fetching device:', error);
      settingsError.value = error instanceof Error ? error.message : t('commonSettings.loadError');
      return null;
    }
  }

  return {
    device,
    settings,
    isLoadingSettings,
    settingsError,
    tokenRequired,
    getDeviceId,
    decodeDeviceId,
    fetchDeviceSettings,
  };
}

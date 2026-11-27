import { scanDevices } from '../api/common';
import type { ScreenConfigs } from '../types/screen';
import type { DivoomDevice } from '../types/device';
import { sendScreen } from '../utils/screenRender';

/**
 * Sends screen configurations to a device. Each screen (image + texts) is
 * rendered on the PC and sent as one picture; empty screens are left alone so
 * the device keeps whatever dial it shows there.
 */
export async function sendConfigsToDevice(
  deviceIp: string,
  screenConfigs: ScreenConfigs
): Promise<void> {
  for (let i = 0; i < 5; i++) {
    const config = screenConfigs[i];
    if (!config || (!config.image && config.texts.length === 0)) continue;

    try {
      await sendScreen(deviceIp, { ...config, screenIndex: i });
    } catch (error) {
      console.error(`Error sending screen ${i} to ${deviceIp}:`, error);
    }
  }
}

/**
 * Gets device ID from device (same logic as useDevice.getDeviceId)
 */
function getDeviceId(device: DivoomDevice): string {
  return encodeURIComponent(
    device.ip_address || device.mac_address || device.name
  );
}

/**
 * Sends saved configurations to all devices that have configs
 */
export async function sendConfigsToAllDevices(): Promise<void> {
  try {
    // Scan for devices
    const devices = await scanDevices();

    if (devices.length === 0) {
      console.log('No devices found, skipping config send');
      return;
    }

    // Send configs to each device that has saved configs
    const sendPromises = devices.map(async (device) => {
      if (!device.ip_address) {
        return; // Skip devices without IP
      }

      const deviceId = getDeviceId(device);
      const stored = localStorage.getItem(`screen_configs_${deviceId}`);

      if (stored) {
        try {
          const screenConfigs: ScreenConfigs = JSON.parse(stored);
          console.log(
            `Sending configs to device ${device.name} (${device.ip_address})`
          );
          await sendConfigsToDevice(device.ip_address, screenConfigs);
          console.log(`Configs sent successfully to ${device.name}`);
        } catch (error) {
          console.error(`Error sending configs to ${device.name}:`, error);
        }
      }
    });

    await Promise.allSettled(sendPromises);
  } catch (error) {
    console.error('Error in sendConfigsToAllDevices:', error);
  }
}

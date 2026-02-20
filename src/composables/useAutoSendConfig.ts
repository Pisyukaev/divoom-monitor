import { invoke } from '@tauri-apps/api/core';
import { scanDevices } from '../api/common';
import type { ScreenConfigs } from '../types/screen';
import type { DivoomDevice } from '../types/device';

/**
 * Sends screen configurations to a device
 */
export async function sendConfigsToDevice(
  deviceIp: string,
  screenConfigs: ScreenConfigs
): Promise<void> {
  // Send images and texts for each screen
  for (let i = 0; i < 5; i++) {
    const config = screenConfigs[i];
    if (!config) continue;

    // Send image if present
    if (config.image) {
      let method = '';

      switch (config.image.type) {
        case 'url':
          method = 'upload_image_from_url';
          break;

        case 'local':
          method = 'upload_image_from_file';
          break;

        default:
          throw new Error(`Invalid image type: ${config.image.type}`);
      }

      const params: Record<string, unknown> = {
          ipAddress: deviceIp,
          screenIndex: i,
        };

      if (config.image.type === 'url') {
        params.url = config.image.source;
      } else if (config.image.type === 'local') {
        params.filePath = config.image.source;
      }

      try {
        await invoke(method, params);
      } catch (error) {
        console.error(
          `Error sending image for screen ${i} to ${deviceIp}:`,
          error
        );
      }
    }

    // Send all texts for this screen
    for (const text of config.texts) {
      try {
        await invoke('set_screen_text', {
          ipAddress: deviceIp,
          screenIndex: i,
          textConfig: {
            id: text.id,
            content: text.content,
            x: text.x,
            y: text.y,
            font: text.font,
            color: text.color?.toUpperCase(),
            alignment: text.alignment,
            text_width: text.textWidth,
          },
        });
      } catch (error) {
        console.error(
          `Error sending text for screen ${i} to ${deviceIp}:`,
          error
        );
      }
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

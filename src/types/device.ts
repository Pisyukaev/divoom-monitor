export interface DivoomDevice {
  name: string;
  mac_address: string | null;
  device_type: string;
  ip_address: string | null;
  signal_strength: number | null;
  is_connected: boolean;
  model: string | null;
}

export interface NetworkSettings {
  ssid?: string;
  ip_address?: string;
  mac_address?: string;
  signal_strength?: number;
}

export interface DeviceSettings {
  brightness?: number;
  volume?: number;
  display_mode?: string;
  current_time?: string;
  network_settings?: NetworkSettings;
  [key: string]: unknown;
}


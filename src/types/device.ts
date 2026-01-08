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
  rotation_flag: number;
  date_format: string;
  time24_flag: number;
  temperature_mode: number;
  mirror_flag: number;
  light_switch: number;
}

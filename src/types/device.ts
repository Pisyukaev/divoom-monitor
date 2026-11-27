export interface DivoomDevice {
  name: string;
  mac_address: string | null;
  device_type: string;
  ip_address: string | null;
  signal_strength: number | null;
  is_connected: boolean;
  device_id: number;
}

export interface NetworkSettings {
  ssid?: string;
  ip_address?: string;
  mac_address?: string;
  signal_strength?: number;
}

/** Mirrors `DeviceSettings` in models.rs: every field is `null` when the device didn't report it. */
export interface DeviceSettings {
  brightness: number | null;
  rotation_flag: number | null;
  date_format: string | null;
  time24_flag: number | null;
  temperature_mode: number | null;
  mirror_flag: number | null;
  light_switch: number | null;
}

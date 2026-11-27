export interface DiskUsage {
  name: string;
  mount_point: string;
  total_space: number;
  available_space: number;
  used_space: number;
  usage_percent: number;
}

export interface SystemMetrics {
  cpu_usage: number;
  cpu_temperature: number | null;
  gpu_usage: number | null;
  gpu_temperature: number | null;
  memory_total: number;
  memory_used: number;
  disks: DiskUsage[];
  /** Why cpu_temperature is null (see the sidecar). */
  cpu_temperature_issue?: 'not_elevated' | 'no_driver' | 'no_sidecar' | null;
}

export interface LcdInfo {
  lcd_clock_id: number;
}

export interface LcdIndependenceInfo {
  lcd_independence: number;
  lcd_list: LcdInfo[];
}

export interface LcdInfoResponse {
  device_id: number;
  /** 0 = whole dial, 1 = independent dials */
  channel_type: number;
  /** Independence dial currently selected on the device */
  lcd_independence: number;
  independence_list: LcdIndependenceInfo[];
}

/** The independence dial the device is showing, falling back to the first one. */
export function selectedIndependence(info: LcdInfoResponse): LcdIndependenceInfo | null {
  return (
    info.independence_list.find((i) => i.lcd_independence === info.lcd_independence) ??
    info.independence_list[0] ??
    null
  );
}

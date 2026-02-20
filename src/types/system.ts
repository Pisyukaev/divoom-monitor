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
  gpu_temperature: number | null;
  memory_total: number;
  memory_used: number;
  disks: DiskUsage[];
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
  independence_list: LcdIndependenceInfo[];
}

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useRoute } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { Refresh, Upload, Connection } from '@element-plus/icons-vue';
import { ElMessage } from 'element-plus';

import { getSystemMetrics, getLcdInfo, activatePcMonitor } from '../api/system';
import {
  savePcMonitorSettings,
  loadPcMonitorSettings,
  startPcMonitorLoop,
  stopPcMonitorLoop,
  isPcMonitorRunning,
} from '../composables/usePcMonitorSend';
import type { DiskUsage, SystemMetrics, LcdInfoResponse, LcdIndependenceInfo } from '../types/system';

const { t } = useI18n();

const route = useRoute();

const deviceIp = computed(() => {
  const decodedId = decodeURIComponent(route.params.id as string);
  if (/^(\d{1,3}\.){3}\d{1,3}$/.test(decodedId)) {
    return decodedId;
  }
  return '';
});

const metrics = ref<SystemMetrics | null>(null);
const isLoading = ref(false);
const error = ref<string | null>(null);
const lastUpdated = ref<Date | null>(null);

const lcdInfo = ref<LcdInfoResponse | null>(null);
const isLoadingLcd = ref(false);
const lcdError = ref<string | null>(null);
const selectedScreen = ref(0);
const autoSendEnabled = ref(false);
const isActivating = ref(false);
const sendError = ref<string | null>(null);

let refreshTimer: number | undefined;
let settingsLoaded = false;

const memoryUsagePercent = computed(() => {
  if (!metrics.value || metrics.value.memory_total === 0) {
    return 0;
  }
  return (metrics.value.memory_used / metrics.value.memory_total) * 100;
});

const disks = computed<DiskUsage[]>(() => metrics.value?.disks ?? []);

const maxDiskUsage = computed(() => {
  if (disks.value.length === 0) return 0;
  return Math.max(...disks.value.map((d) => d.usage_percent));
});

const currentIndependence = computed<LcdIndependenceInfo | null>(() => {
  if (!lcdInfo.value || lcdInfo.value.independence_list.length === 0) {
    return null;
  }
  return lcdInfo.value.independence_list[0];
});

const screenOptions = computed(() => {
  const independence = currentIndependence.value;
  if (!independence || independence.lcd_list.length === 0) {
    return Array.from({ length: 5 }, (_, i) => ({
      value: i,
      label: t('systemMetrics.screenN', { n: i + 1 }),
    }));
  }

  return independence.lcd_list.map((lcd, i) => ({
    value: i,
    label: lcd.lcd_clock_id === 625
      ? t('systemMetrics.screenPcMonitor', { n: i + 1 })
      : t('systemMetrics.screenClock', { n: i + 1, clockId: lcd.lcd_clock_id }),
  }));
});

const isSelectedScreenPcMonitor = computed(() => {
  const independence = currentIndependence.value;
  if (!independence) return false;
  const lcd = independence.lcd_list[selectedScreen.value];
  return lcd?.lcd_clock_id === 625;
});

const formatBytes = (bytes: number) => {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
};

const formatTemperature = (value: number | null) => {
  if (value === null || Number.isNaN(value)) return '—';
  return `${value.toFixed(1)} °C`;
};

const formatPercent = (value: number) => `${value.toFixed(1)}%`;

const loadMetrics = async () => {
  isLoading.value = true;
  error.value = null;
  try {
    metrics.value = await getSystemMetrics();
    lastUpdated.value = new Date();
  } catch (err) {
    error.value = err instanceof Error ? err.message : t('systemMetrics.metricsError');
  } finally {
    isLoading.value = false;
  }
};

async function loadLcdInfo() {
  if (!deviceIp.value) return;

  isLoadingLcd.value = true;
  lcdError.value = null;
  try {
    lcdInfo.value = await getLcdInfo(deviceIp.value);
  } catch (err) {
    lcdError.value = err instanceof Error ? err.message : t('systemMetrics.lcdError');
  } finally {
    isLoadingLcd.value = false;
  }
}

async function handleActivate() {
  if (!deviceIp.value || !lcdInfo.value) return;

  const independence = currentIndependence.value;
  const lcdIndependence = independence?.lcd_independence ?? 0;

  isActivating.value = true;
  try {
    await activatePcMonitor(
      deviceIp.value,
      lcdInfo.value.device_id,
      lcdIndependence,
      selectedScreen.value,
    );
    ElMessage.success(t('systemMetrics.pcMonitorActivated', { n: selectedScreen.value + 1 }));
    await loadLcdInfo();
  } catch (err) {
    ElMessage.error(t('systemMetrics.activationError', { error: String(err) }));
  } finally {
    isActivating.value = false;
  }
}

function persistAndSync(enabled: boolean, lcdIndex: number) {
  if (!deviceIp.value) return;

  savePcMonitorSettings(deviceIp.value, { lcdIndex, enabled });

  if (enabled) {
    startPcMonitorLoop(deviceIp.value, lcdIndex);
  } else {
    stopPcMonitorLoop(deviceIp.value);
  }
}

watch(autoSendEnabled, (enabled) => {
  if (!settingsLoaded) return;
  sendError.value = null;
  persistAndSync(enabled, selectedScreen.value);
});

watch(selectedScreen, (newIndex) => {
  if (!settingsLoaded || !autoSendEnabled.value) return;
  persistAndSync(true, newIndex);
});

onMounted(() => {
  if (deviceIp.value) {
    const saved = loadPcMonitorSettings(deviceIp.value);
    if (saved) {
      selectedScreen.value = saved.lcdIndex;
      autoSendEnabled.value = saved.enabled && isPcMonitorRunning(deviceIp.value);
    }
  }
  settingsLoaded = true;

  loadMetrics();
  loadLcdInfo();
  refreshTimer = window.setInterval(loadMetrics, 2000);
});

onUnmounted(() => {
  if (refreshTimer) {
    window.clearInterval(refreshTimer);
  }
});
</script>

<template>
  <div class="metrics-page">
    <div class="metrics-header">
      <div>
        <h2>{{ t('systemMetrics.title') }}</h2>
        <p class="subtitle">{{ t('systemMetrics.subtitle') }}</p>
      </div>
      <div class="header-actions">
        <span v-if="lastUpdated" class="timestamp">
          {{ t('systemMetrics.updated', { time: lastUpdated.toLocaleTimeString() }) }}
        </span>
        <el-button :icon="Refresh" :loading="isLoading" @click="loadMetrics">
          {{ t('systemMetrics.refresh') }}
        </el-button>
      </div>
    </div>

    <el-alert v-if="error" type="error" :title="error" show-icon :closable="false" />

    <div v-if="metrics" class="metrics-grid">
      <el-card class="metric-card">
        <template #header>
          <span>{{ t('systemMetrics.cpu') }}</span>
        </template>
        <div class="metric-value">
          <span class="value">{{ formatPercent(metrics.cpu_usage) }}</span>
          <span class="label">{{ t('systemMetrics.load') }}</span>
        </div>
        <el-progress :percentage="metrics.cpu_usage" :stroke-width="10" :format="formatPercent" />
        <div class="metric-footer">
          <span>{{ t('systemMetrics.temperature') }}</span>
          <strong>{{ formatTemperature(metrics.cpu_temperature) }}</strong>
        </div>
      </el-card>

      <el-card class="metric-card">
        <template #header>
          <span>{{ t('systemMetrics.gpu') }}</span>
        </template>
        <div class="metric-value">
          <span class="value">{{ formatTemperature(metrics.gpu_temperature) }}</span>
          <span class="label">{{ t('systemMetrics.temperature') }}</span>
        </div>
        <p class="hint">{{ t('systemMetrics.gpuHint') }}</p>
      </el-card>

      <el-card class="metric-card">
        <template #header>
          <span>{{ t('systemMetrics.ram') }}</span>
        </template>
        <div class="metric-value">
          <span class="value">{{ formatPercent(memoryUsagePercent) }}</span>
          <span class="label">{{ t('systemMetrics.usage') }}</span>
        </div>
        <el-progress :percentage="memoryUsagePercent" :stroke-width="10" status="success" :format="formatPercent" />
        <div class="metric-footer">
          <span>{{ formatBytes(metrics.memory_used) }}</span>
          <span>{{ t('systemMetrics.of') }} {{ formatBytes(metrics.memory_total) }}</span>
        </div>
      </el-card>
    </div>

    <el-card v-if="metrics" class="disk-card">
      <template #header>
        <span>{{ t('systemMetrics.disks') }}</span>
      </template>
      <div v-if="disks.length === 0" class="empty-state">{{ t('systemMetrics.disksUnavailable') }}</div>
      <div v-else class="disk-list">
        <div v-for="disk in disks" :key="`${disk.name}-${disk.mount_point}`" class="disk-item">
          <strong>{{ disk.name || t('systemMetrics.disk') }}</strong>
          <el-progress :percentage="disk.usage_percent" :stroke-width="8" :format="formatPercent" />
          <div class="disk-info">
            <div class="metric-footer">
              <span>{{ formatBytes(disk.used_space) }} {{ t('systemMetrics.of') }} {{ formatBytes(disk.total_space) }}</span>
            </div>
          </div>
        </div>
      </div>
    </el-card>

    <el-card v-if="deviceIp" class="device-send-card" v-loading="isLoadingLcd">
      <template #header>
        <div class="send-header">
          <span>{{ t('systemMetrics.sendToDevice') }}</span>
          <el-button :icon="Refresh" size="small" circle @click="loadLcdInfo" :loading="isLoadingLcd" />
        </div>
      </template>

      <el-alert v-if="lcdError" type="warning" :title="lcdError" show-icon :closable="false"
        style="margin-bottom: 16px" />

      <div class="send-controls">
        <div class="send-row">
          <label class="send-label">{{ t('systemMetrics.screen') }}</label>
          <el-select v-model="selectedScreen" style="width: 260px" :disabled="autoSendEnabled">
            <el-option v-for="opt in screenOptions" :key="opt.value" :label="opt.label" :value="opt.value" />
          </el-select>
        </div>

        <div class="send-row">
          <label class="send-label">{{ t('systemMetrics.pcMonitorMode') }}</label>
          <div class="send-actions">
            <el-button type="primary" :icon="Connection" :loading="isActivating" :disabled="isSelectedScreenPcMonitor"
              @click="handleActivate">
              {{ isSelectedScreenPcMonitor ? t('systemMetrics.alreadyActivated') : t('systemMetrics.activate') }}
            </el-button>
            <el-tag v-if="isSelectedScreenPcMonitor" type="success" size="small">Clock 625</el-tag>
          </div>
        </div>

        <div class="send-row">
          <label class="send-label">{{ t('systemMetrics.autoSendMetrics') }}</label>
          <el-switch v-model="autoSendEnabled" :active-text="t('systemMetrics.on')" :inactive-text="t('systemMetrics.off')" />
        </div>

        <el-alert v-if="sendError" type="error" :title="sendError" show-icon :closable="true"
          @close="sendError = null" style="margin-top: 8px" />

        <div v-if="autoSendEnabled" class="send-status">
          <el-icon color="var(--el-color-success)">
            <Upload />
          </el-icon>
          <span>{{ t('systemMetrics.metricsSending') }}</span>
        </div>

        <div v-if="metrics && autoSendEnabled" class="send-preview">
          <p class="send-preview-title">{{ t('systemMetrics.sentData') }}</p>
          <div class="send-preview-grid">
            <span>CPU: {{ Math.round(metrics.cpu_usage) }}%</span>
            <span>GPU: 0%</span>
            <span>CPU t: {{ metrics.cpu_temperature !== null ? `${Math.round(metrics.cpu_temperature)} C` : 'N/A'
              }}</span>
            <span>GPU t: {{ metrics.gpu_temperature !== null ? `${Math.round(metrics.gpu_temperature)} C` : 'N/A'
              }}</span>
            <span>RAM: {{ Math.round(memoryUsagePercent) }}%</span>
            <span>HDD: {{ Math.round(maxDiskUsage) }}%</span>
          </div>
        </div>
      </div>
    </el-card>
  </div>
</template>

<style scoped>
.metrics-page {
  max-width: 1200px;
  margin: 0 auto;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.metrics-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 20px;
  flex-wrap: wrap;
}

.metrics-header h2 {
  margin: 0;
  font-size: 28px;
  color: var(--el-text-color-primary);
}

.subtitle {
  margin: 4px 0 0;
  color: var(--el-text-color-secondary);
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 16px;
}

.timestamp {
  color: var(--el-text-color-secondary);
  font-size: 14px;
}

.metrics-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  gap: 20px;
}

.metric-card {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.metric-value {
  display: flex;
  align-items: baseline;
  gap: 10px;
}

.metric-value .value {
  font-size: 32px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.metric-value .label {
  color: var(--el-text-color-secondary);
}

.metric-footer {
  display: flex;
  justify-content: space-between;
  color: var(--el-text-color-secondary);
  font-size: 14px;
}

.hint {
  margin: 0;
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

.disk-card {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.disk-list {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.disk-item {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.disk-info {
  display: flex;
  justify-content: end;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}

.empty-state {
  color: var(--el-text-color-secondary);
}

.device-send-card {
  display: flex;
  flex-direction: column;
}

.send-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.send-controls {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.send-row {
  display: flex;
  align-items: center;
  gap: 16px;
}

.send-label {
  min-width: 180px;
  font-size: 14px;
  color: var(--el-text-color-primary);
  font-weight: 500;
}

.send-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.send-status {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  background-color: var(--el-fill-color-light);
  border-radius: 6px;
  font-size: 13px;
  color: var(--el-text-color-secondary);
}

.send-preview {
  padding: 12px 14px;
  background-color: var(--el-fill-color-lighter);
  border-radius: 6px;
  border: 1px solid var(--el-border-color-lighter);
}

.send-preview-title {
  margin: 0 0 8px;
  font-size: 13px;
  color: var(--el-text-color-secondary);
}

.send-preview-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 6px 16px;
  font-size: 13px;
  font-family: monospace;
  color: var(--el-text-color-primary);
}

@media (max-width: 768px) {
  .send-row {
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
  }

  .send-label {
    min-width: auto;
  }

  .send-preview-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}
</style>

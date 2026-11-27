<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useRoute } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { Refresh, Connection, Promotion } from '@element-plus/icons-vue';
import SettingRow from '../components/ui/SettingRow.vue';
import { ElMessage } from 'element-plus';

import { getSystemMetrics, getLcdInfo, activatePcMonitor } from '../api/system';
import {
  savePcMonitorSettings,
  loadPcMonitorSettings,
  startPcMonitorLoop,
  stopPcMonitorLoop,
  isPcMonitorRunning,
} from '../composables/usePcMonitorSend';
import {
  selectedIndependence,
  type DiskUsage,
  type SystemMetrics,
  type LcdInfoResponse,
  type LcdIndependenceInfo,
} from '../types/system';

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

const currentIndependence = computed<LcdIndependenceInfo | null>(() => {
  return lcdInfo.value ? selectedIndependence(lcdInfo.value) : null;
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

const gaugeColor = (value: number) =>
  value >= 85 ? 'var(--app-danger)' : value >= 65 ? 'var(--app-warning)' : 'var(--el-color-primary)';

const tempClass = (value: number | null) =>
  value === null ? '' : value >= 85 ? 'is-hot' : value >= 70 ? 'is-warm' : '';

const screenButtons = computed(() =>
  screenOptions.value.map((opt, i) => ({
    index: opt.value,
    isPcMonitor: currentIndependence.value?.lcd_list[i]?.lcd_clock_id === 625,
  })),
);

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
  <div class="monitor">
    <section v-if="deviceIp" v-loading="isLoadingLcd" class="app-card">
      <div class="app-card-header">
        <div class="title-row">
          <span class="app-card-title">{{ t('systemMetrics.sendToDevice') }}</span>
          <span class="status-pill" :class="{ active: autoSendEnabled }">
            <span class="app-status-dot" :class="{ 'is-online': autoSendEnabled }" />
            {{ autoSendEnabled ? t('systemMetrics.statusSending') : t('systemMetrics.statusIdle') }}
          </span>
        </div>
        <el-tooltip :content="t('systemMetrics.refreshScreens')" placement="top">
          <el-button :icon="Refresh" text circle :loading="isLoadingLcd" @click="loadLcdInfo" />
        </el-tooltip>
      </div>

      <div class="app-card-body">
        <el-alert v-if="lcdError" type="warning" :title="lcdError" show-icon :closable="false" class="lcd-alert" />

        <div class="screen-picker-label app-section-label">{{ t('systemMetrics.screen') }}</div>
        <div class="screen-picker">
          <button v-for="btn in screenButtons" :key="btn.index" class="screen-btn"
            :class="{ selected: selectedScreen === btn.index }" :disabled="autoSendEnabled"
            @click="selectedScreen = btn.index">
            <span class="screen-num">{{ btn.index + 1 }}</span>
            <span v-if="btn.isPcMonitor" class="screen-badge">PC</span>
          </button>
        </div>

        <SettingRow :icon="Connection" :title="t('systemMetrics.pcMonitorMode')"
          :description="t('systemMetrics.pcMonitorModeDesc')">
          <el-tag v-if="isSelectedScreenPcMonitor" type="success" effect="light" round>
            {{ t('systemMetrics.alreadyActivated') }}
          </el-tag>
          <el-button v-else type="primary" plain :loading="isActivating" @click="handleActivate">
            {{ t('systemMetrics.activate') }}
          </el-button>
        </SettingRow>

        <SettingRow :icon="Promotion" :title="t('systemMetrics.autoSendMetrics')"
          :description="t('systemMetrics.metricsSending')">
          <el-switch v-model="autoSendEnabled" />
        </SettingRow>

        <el-alert v-if="sendError" type="error" :title="sendError" show-icon :closable="true"
          @close="sendError = null" />
      </div>
    </section>

    <div class="section-head">
      <span class="app-section-label">{{ t('systemMetrics.title') }}</span>
      <span v-if="lastUpdated" class="app-muted timestamp">
        {{ t('systemMetrics.updated', { time: lastUpdated.toLocaleTimeString() }) }}
      </span>
    </div>

    <el-alert v-if="error" type="error" :title="error" show-icon :closable="false" />

    <div v-if="metrics" class="gauges">
      <div class="gauge app-card">
        <el-progress type="dashboard" :percentage="Math.round(metrics.cpu_usage)" :width="128" :stroke-width="10"
          :color="gaugeColor(metrics.cpu_usage)">
          <template #default="{ percentage }">
            <span class="gauge-value">{{ percentage }}<small>%</small></span>
            <span class="gauge-label">{{ t('systemMetrics.cpu') }}</span>
          </template>
        </el-progress>
        <div class="gauge-foot">
          <span class="app-muted">{{ t('systemMetrics.temperature') }}</span>
          <strong :class="tempClass(metrics.cpu_temperature)">{{ formatTemperature(metrics.cpu_temperature) }}</strong>
        </div>
        <p v-if="metrics.cpu_temperature === null && metrics.cpu_temperature_issue" class="app-muted hint">
          {{ t(`systemMetrics.cpuTempIssue.${metrics.cpu_temperature_issue}`) }}
        </p>
      </div>

      <div class="gauge app-card">
        <el-progress type="dashboard" :percentage="Math.round(metrics.gpu_usage ?? 0)" :width="128" :stroke-width="10"
          :color="gaugeColor(metrics.gpu_usage ?? 0)">
          <template #default>
            <span class="gauge-value">
              <template v-if="metrics.gpu_usage !== null">{{ Math.round(metrics.gpu_usage) }}<small>%</small></template>
              <template v-else>—</template>
            </span>
            <span class="gauge-label">{{ t('systemMetrics.gpu') }}</span>
          </template>
        </el-progress>
        <div class="gauge-foot">
          <span class="app-muted">{{ t('systemMetrics.temperature') }}</span>
          <strong :class="tempClass(metrics.gpu_temperature)">{{ formatTemperature(metrics.gpu_temperature) }}</strong>
        </div>
        <p v-if="metrics.gpu_usage === null && metrics.gpu_temperature === null" class="app-muted hint">
          {{ t('systemMetrics.gpuHint') }}
        </p>
      </div>

      <div class="gauge app-card">
        <el-progress type="dashboard" :percentage="Math.round(memoryUsagePercent)" :width="128" :stroke-width="10"
          :color="gaugeColor(memoryUsagePercent)">
          <template #default="{ percentage }">
            <span class="gauge-value">{{ percentage }}<small>%</small></span>
            <span class="gauge-label">{{ t('systemMetrics.ram') }}</span>
          </template>
        </el-progress>
        <div class="gauge-foot">
          <span class="app-muted">{{ t('systemMetrics.used') }}</span>
          <strong>{{ formatBytes(metrics.memory_used) }} / {{ formatBytes(metrics.memory_total) }}</strong>
        </div>
      </div>
    </div>

    <section v-if="metrics" class="app-card">
      <div class="app-card-header">
        <span class="app-card-title">{{ t('systemMetrics.disks') }}</span>
      </div>
      <div class="app-card-body">
        <div v-if="disks.length === 0" class="app-muted">{{ t('systemMetrics.disksUnavailable') }}</div>
        <div v-else class="disk-list">
          <div v-for="disk in disks" :key="`${disk.name}-${disk.mount_point}`" class="disk">
            <div class="disk-head">
              <strong>{{ disk.name || t('systemMetrics.disk') }}</strong>
              <span class="app-mono app-muted">{{ disk.mount_point }}</span>
              <span class="disk-size app-muted">
                {{ formatBytes(disk.used_space) }} {{ t('systemMetrics.of') }} {{ formatBytes(disk.total_space) }}
              </span>
            </div>
            <el-progress :percentage="Math.round(disk.usage_percent)" :stroke-width="8"
              :color="gaugeColor(disk.usage_percent)" />
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.monitor {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.title-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.status-pill {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 2px 10px;
  border-radius: 999px;
  font-size: 12px;
  font-weight: 500;
  background: var(--el-fill-color-light);
  color: var(--el-text-color-secondary);
}

.status-pill.active {
  background: color-mix(in srgb, var(--app-success) 14%, transparent);
  color: var(--app-success);
}

.lcd-alert {
  margin-bottom: 12px;
}

.screen-picker-label {
  margin-bottom: 8px;
}

.screen-picker {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  padding: 4px 0 8px;
}

.screen-btn {
  position: relative;
  width: 56px;
  height: 56px;
  border-radius: 12px;
  border: 1.5px solid var(--el-border-color);
  background: var(--app-surface-muted);
  color: var(--el-text-color-primary);
  font: inherit;
  cursor: pointer;
  transition: border-color 0.15s, background-color 0.15s;
}

.screen-btn:hover:not(:disabled) {
  border-color: var(--el-color-primary-light-5);
}

.screen-btn.selected {
  border-color: var(--el-color-primary);
  background: var(--el-color-primary-light-9);
  color: var(--el-color-primary);
}

.screen-btn:disabled {
  cursor: not-allowed;
  opacity: 0.6;
}

.screen-num {
  font-size: 18px;
  font-weight: 600;
}

.screen-badge {
  position: absolute;
  top: -7px;
  right: -7px;
  padding: 0 6px;
  border-radius: 999px;
  font-size: 10px;
  font-weight: 700;
  line-height: 16px;
  background: var(--app-success);
  color: #fff;
}

.section-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  margin-top: 8px;
}

.timestamp {
  font-size: 12.5px;
}

.gauges {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 16px;
}

.gauge {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 20px 20px 16px;
}

.gauge :deep(.el-progress__text) {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.gauge-value {
  font-size: 26px;
  font-weight: 650;
  color: var(--el-text-color-primary);
  font-variant-numeric: tabular-nums;
}

.gauge-value small {
  font-size: 14px;
  font-weight: 500;
  margin-left: 1px;
}

.gauge-label {
  display: block;
  margin-top: 2px;
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.06em;
  color: var(--el-text-color-secondary);
}

.gauge-foot {
  align-self: stretch;
  display: flex;
  justify-content: space-between;
  gap: 8px;
  padding-top: 12px;
  border-top: 1px solid var(--el-border-color-lighter);
  font-size: 13px;
  font-variant-numeric: tabular-nums;
}

.is-warm {
  color: var(--app-warning);
}

.is-hot {
  color: var(--app-danger);
}

.hint {
  margin: 0;
  font-size: 12px;
  text-align: center;
}

.disk-list {
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.disk-head {
  display: flex;
  align-items: baseline;
  gap: 10px;
  margin-bottom: 6px;
}

.disk-size {
  margin-left: auto;
  font-size: 13px;
  font-variant-numeric: tabular-nums;
}
</style>

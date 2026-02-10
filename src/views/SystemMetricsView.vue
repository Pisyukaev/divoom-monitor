<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { Refresh } from '@element-plus/icons-vue';

import Header from '../components/Header.vue';
import { getSystemMetrics } from '../api/system';
import type { DiskUsage, SystemMetrics } from '../types/system';

const metrics = ref<SystemMetrics | null>(null);
const isLoading = ref(false);
const error = ref<string | null>(null);
const lastUpdated = ref<Date | null>(null);

let refreshTimer: number | undefined;

const memoryUsagePercent = computed(() => {
  if (!metrics.value || metrics.value.memory_total === 0) {
    return 0;
  }
  return (metrics.value.memory_used / metrics.value.memory_total) * 100;
});

const disks = computed<DiskUsage[]>(() => metrics.value?.disks ?? []);

const formatBytes = (bytes: number) => {
  if (bytes === 0) {
    return '0 B';
  }
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
};

const formatTemperature = (value: number | null) => {
  if (value === null || Number.isNaN(value)) {
    return '—';
  }
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
    error.value = err instanceof Error ? err.message : 'Не удалось получить метрики системы';
  } finally {
    isLoading.value = false;
  }
};

onMounted(() => {
  loadMetrics();
  refreshTimer = window.setInterval(loadMetrics, 2000);
});

onUnmounted(() => {
  if (refreshTimer) {
    window.clearInterval(refreshTimer);
  }
});
</script>

<template>
  <Header />
  <div class="metrics-page">
    <div class="metrics-header">
      <div>
        <h2>Состояние системы</h2>
        <p class="subtitle">Актуальные показатели загрузки и температуры</p>
      </div>
      <div class="header-actions">
        <span v-if="lastUpdated" class="timestamp">
          Обновлено: {{ lastUpdated.toLocaleTimeString() }}
        </span>
        <el-button :icon="Refresh" :loading="isLoading" @click="loadMetrics">
          Обновить
        </el-button>
      </div>
    </div>

    <el-alert v-if="error" type="error" :title="error" show-icon :closable="false" />

    <div v-if="metrics" class="metrics-grid">
      <el-card class="metric-card">
        <template #header>
          <span>CPU</span>
        </template>
        <div class="metric-value">
          <span class="value">{{ formatPercent(metrics.cpu_usage) }}</span>
          <span class="label">Загрузка</span>
        </div>
        <el-progress :percentage="metrics.cpu_usage" :stroke-width="10" />
        <div class="metric-footer">
          <span>Температура</span>
          <strong>{{ formatTemperature(metrics.cpu_temperature) }}</strong>
        </div>
      </el-card>

      <el-card class="metric-card">
        <template #header>
          <span>GPU</span>
        </template>
        <div class="metric-value">
          <span class="value">{{ formatTemperature(metrics.gpu_temperature) }}</span>
          <span class="label">Температура</span>
        </div>
        <p class="hint">Если температура не доступна, устройство не сообщает датчик.</p>
      </el-card>

      <el-card class="metric-card">
        <template #header>
          <span>RAM</span>
        </template>
        <div class="metric-value">
          <span class="value">{{ formatPercent(memoryUsagePercent) }}</span>
          <span class="label">Использование</span>
        </div>
        <el-progress :percentage="memoryUsagePercent" :stroke-width="10" status="success" />
        <div class="metric-footer">
          <span>{{ formatBytes(metrics.memory_used) }}</span>
          <span>из {{ formatBytes(metrics.memory_total) }}</span>
        </div>
      </el-card>
    </div>

    <el-card v-if="metrics" class="disk-card">
      <template #header>
        <span>Диски</span>
      </template>
      <div v-if="disks.length === 0" class="empty-state">Данные о дисках недоступны.</div>
      <div v-else class="disk-list">
        <div v-for="disk in disks" :key="`${disk.name}-${disk.mount_point}`" class="disk-item">
          <div class="disk-info">
            <div class="disk-title">
              <strong>{{ disk.name || 'Диск' }}</strong>
              <span class="disk-mount">{{ disk.mount_point }}</span>
            </div>
            <div class="disk-metrics">
              <span>{{ formatBytes(disk.used_space) }} из {{ formatBytes(disk.total_space) }}</span>
              <span>{{ formatPercent(disk.usage_percent) }}</span>
            </div>
          </div>
          <el-progress :percentage="disk.usage_percent" :stroke-width="8" />
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
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}

.disk-title {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.disk-mount {
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

.disk-metrics {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 4px;
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

.empty-state {
  color: var(--el-text-color-secondary);
}
</style>

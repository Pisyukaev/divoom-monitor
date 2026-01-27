<script setup lang="ts">
import type { DivoomDevice } from '../../types/device';

const props = defineProps<{
  device: DivoomDevice;
  onClick: () => void
}>();



function getSignalBarActive(
  barIndex: number,
  signalStrength: number | null
): boolean {
  if (signalStrength === null) return false;
  // Signal strength is negative, so -30 is better than -90
  // Map to 4 bars: -30 to -50 = 4 bars, -50 to -70 = 3 bars, etc.
  const threshold = -30 - barIndex * 20;
  return signalStrength >= threshold;
}
</script>

<template>
  <el-card :shadow="device.is_connected ? 'always' : 'hover'" :class="{
    'device-card-connected': device.is_connected,
    'device-card-clickable': true,
  }" @click="onClick">
    <template #header>
      <div class="device-header">
        <h3 class="device-name">{{ device.name }}</h3>
        <el-tag :type="device.is_connected ? 'success' : 'danger'" size="small">
          {{ device.is_connected ? 'Подключено' : 'Не подключено' }}
        </el-tag>
      </div>
    </template>

    <el-descriptions :column="1" border size="small">
      <el-descriptions-item label="Тип устройства">
        {{ device.device_type }}
      </el-descriptions-item>

      <el-descriptions-item v-if="device.ip_address" label="IP адрес">
        <el-text>{{ device.ip_address }}</el-text>
      </el-descriptions-item>

      <el-descriptions-item v-if="device.mac_address" label="MAC адрес">
        <el-text>{{ device.mac_address }}</el-text>
      </el-descriptions-item>

      <el-descriptions-item v-if="device.device_id" label="ID">
        <el-text>{{ device.device_id }}</el-text>
      </el-descriptions-item>

      <el-descriptions-item v-if="device.signal_strength !== null" label="Уровень сигнала">
        <div class="signal-indicator">
          <el-text>{{ device.signal_strength }} dBm</el-text>
          <div class="signal-bars">
            <div v-for="i in 4" :key="i" class="signal-bar" :class="{
              active: getSignalBarActive(i, device.signal_strength),
            }"></div>
          </div>
        </div>
      </el-descriptions-item>
    </el-descriptions>
  </el-card>
</template>

<style scoped>
.device-card-connected {
  border: 2px solid var(--el-color-success);
}

.device-card-clickable {
  cursor: pointer;
  transition: transform 0.2s, box-shadow 0.2s;
}

.device-card-clickable:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
}

.device-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.device-name {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.signal-indicator {
  display: flex;
  align-items: center;
  gap: 8px;
}

.signal-bars {
  display: flex;
  gap: 3px;
  align-items: flex-end;
}

.signal-bar {
  width: 4px;
  background-color: var(--el-border-color-light);
  border-radius: 2px;
  transition: background-color 0.3s;
}

.signal-bar:nth-child(1) {
  height: 6px;
}

.signal-bar:nth-child(2) {
  height: 10px;
}

.signal-bar:nth-child(3) {
  height: 14px;
}

.signal-bar:nth-child(4) {
  height: 18px;
}

.signal-bar.active {
  background-color: var(--el-color-success);
}
</style>

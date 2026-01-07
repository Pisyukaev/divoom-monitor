<script lang="ts" setup>
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { Search } from '@element-plus/icons-vue';
import type { DivoomDevice } from '../../types/device';

import DeviceCard from './device-card.vue';

const devices = ref<DivoomDevice[]>([]);
const isScanning = ref(false);
const error = ref<string | null>(null);

async function scanDevices() {
  isScanning.value = true;
  error.value = null;
  try {
    const foundDevices = await invoke<DivoomDevice[]>('scan_devices');
    devices.value = foundDevices;
    console.log(foundDevices);
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'Failed to scan devices';
    console.error('Error scanning devices:', err);
  } finally {
    isScanning.value = false;
  }
}

onMounted(() => {
  scanDevices();
});
</script>

<template>
  <div class="container">
    <div class="controls">
      <el-button
        type="primary"
        :icon="Search"
        :loading="isScanning"
        @click="scanDevices"
        size="large"
      >
        {{ isScanning ? 'Сканирование...' : 'Сканировать устройства' }}
      </el-button>
    </div>

    <el-alert
      v-if="error"
      :title="error"
      type="error"
      :closable="false"
      show-icon
      style="margin-bottom: 20px"
    />

    <el-empty
      v-if="devices.length === 0 && !isScanning"
      description="Устройства не найдены. Нажмите 'Сканировать устройства' для поиска."
    />

    <div class="devices-grid">
      <DeviceCard
        v-for="device in devices"
        :key="device.ip_address || device.mac_address || device.name"
        :device="device"
      />
    </div>
  </div>
</template>

<style scoped>
.container {
  max-width: 1200px;
  margin: 0 auto;
  padding: 20px;
}

.controls {
  display: flex;
  justify-content: center;
  margin-bottom: 20px;
}

.devices-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 20px;
  margin-top: 20px;
}
</style>

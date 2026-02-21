<script lang="ts" setup>
import { ref, onMounted } from 'vue';
import { useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { Search } from '@element-plus/icons-vue';

import { scanDevices } from '../../api/common';
import Header from '../Header.vue';
import DeviceCard from './device-card.vue';
import type { DivoomDevice } from '../../types/device';

const router = useRouter();
const { t } = useI18n();

const devices = ref<DivoomDevice[]>([]);
const isScanning = ref(false);
const error = ref<string | null>(null);

async function scan() {
  isScanning.value = true;
  error.value = null;
  try {
    const foundDevices = await scanDevices();
    devices.value = foundDevices;
  } catch (err) {
    error.value = err instanceof Error ? err.message : t('deviceList.scanFailed');
    console.error('Error scanning devices:', err);
  } finally {
    isScanning.value = false;
  }
}

function handleCardClick(device: DivoomDevice) {
  router.push(`/device/${device.ip_address}`);
}

onMounted(() => {
  scan();
});
</script>

<template>
  <Header />
  <div class="container">
    <div class="controls">
      <el-button type="primary" :icon="Search" :loading="isScanning" @click="scan" size="large">
        {{ isScanning ? t('deviceList.scanning') : t('deviceList.scanDevices') }}
      </el-button>
    </div>

    <el-alert v-if="error" :title="error" type="error" :closable="false" show-icon style="margin-bottom: 20px" />

    <el-empty v-if="devices.length === 0 && !isScanning" :description="t('deviceList.noDevices')" />

    <div class="devices-grid">
      <DeviceCard v-for="device in devices" :key="device.ip_address || device.mac_address || device.name"
        :device="device" :onClick="() => handleCardClick(device)" />
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

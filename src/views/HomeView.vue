<script setup lang="ts">
import { onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { Refresh, ArrowRight, Search } from '@element-plus/icons-vue';

import { useDevices, getDeviceRouteId, isTimesGate } from '../composables/useDevices';

const { t } = useI18n();
const { devices, isScanning, hasScanned, scan, ensureScanned } = useDevices();

onMounted(() => {
  ensureScanned();
});
</script>

<template>
  <div class="app-page">
    <header class="app-page-header">
      <div>
        <h1 class="app-page-title">{{ t('home.title') }}</h1>
        <p class="app-page-subtitle">
          <template v-if="devices.length">{{ t('home.found', devices.length) }}</template>
          <template v-else>{{ t('home.subtitle') }}</template>
        </p>
      </div>
      <el-button type="primary" :icon="Refresh" :loading="isScanning" @click="scan">
        {{ isScanning ? t('home.scanning') : t('home.scan') }}
      </el-button>
    </header>

    <div v-if="devices.length" class="device-grid">
      <router-link v-for="device in devices" :key="getDeviceRouteId(device)" :to="`/device/${getDeviceRouteId(device)}`"
        class="device-card app-card">
        <div class="device-visual">
          <div class="screens" :class="{ multi: isTimesGate(device) }">
            <span v-for="n in isTimesGate(device) ? 5 : 1" :key="n" class="screen" />
          </div>
        </div>
        <div class="device-body">
          <div class="device-title-row">
            <span class="device-name">{{ device.name }}</span>
            <span class="app-status-dot" :class="{ 'is-online': device.is_connected }" />
          </div>
          <div class="app-muted">{{ device.device_type }}</div>
          <dl class="device-props">
            <template v-if="device.ip_address">
              <dt>IP</dt>
              <dd class="app-mono">{{ device.ip_address }}</dd>
            </template>
            <template v-if="device.mac_address">
              <dt>MAC</dt>
              <dd class="app-mono">{{ device.mac_address }}</dd>
            </template>
          </dl>
        </div>
        <div class="device-footer">
          <span>{{ t('home.open') }}</span>
          <el-icon><ArrowRight /></el-icon>
        </div>
      </router-link>
    </div>

    <div v-else class="empty app-card">
      <div class="empty-icon" :class="{ pulsing: isScanning || !hasScanned }">
        <el-icon><Search /></el-icon>
      </div>
      <template v-if="isScanning || !hasScanned">
        <h2 class="empty-title">{{ t('home.searchingTitle') }}</h2>
        <p class="app-muted">{{ t('home.searchingText') }}</p>
      </template>
      <template v-else>
        <h2 class="empty-title">{{ t('home.emptyTitle') }}</h2>
        <ul class="empty-hints">
          <li>{{ t('home.hintNetwork') }}</li>
          <li>{{ t('home.hintAccount') }}</li>
          <li>{{ t('home.hintRetry') }}</li>
        </ul>
        <el-button type="primary" :icon="Refresh" @click="scan">{{ t('home.scan') }}</el-button>
      </template>
    </div>
  </div>
</template>

<style scoped>
.device-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 16px;
}

.device-card {
  display: flex;
  flex-direction: column;
  overflow: hidden;
  color: inherit;
  text-decoration: none;
  transition: border-color 0.15s, transform 0.15s, box-shadow 0.15s;
}

.device-card:hover {
  border-color: var(--el-color-primary-light-5);
  transform: translateY(-2px);
  box-shadow: 0 8px 24px rgba(16, 24, 40, 0.08);
}

.device-visual {
  height: 112px;
  display: flex;
  align-items: center;
  justify-content: center;
  background:
    radial-gradient(circle at 30% 20%, rgba(124, 124, 240, 0.25), transparent 60%),
    var(--app-screen-bezel);
}

.screens {
  display: flex;
  gap: 8px;
}

.screen {
  width: 56px;
  height: 56px;
  border-radius: 6px;
  background-color: #1a1c22;
  background-image: radial-gradient(rgba(255, 255, 255, 0.14) 1px, transparent 1.2px);
  background-size: 6px 6px;
  box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.06);
}

.screens.multi .screen {
  width: 40px;
  height: 40px;
  border-radius: 6px;
}

.device-body {
  padding: 16px 18px 8px;
  flex: 1;
}

.device-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.device-name {
  font-size: 16px;
  font-weight: 600;
}

.device-props {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 4px 12px;
  margin: 12px 0 0;
  font-size: 13px;
}

.device-props dt {
  color: var(--el-text-color-secondary);
}

.device-props dd {
  margin: 0;
}

.device-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 18px;
  border-top: 1px solid var(--el-border-color-lighter);
  color: var(--el-color-primary);
  font-weight: 500;
}

.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  gap: 8px;
  padding: 56px 24px;
}

.empty-icon {
  width: 64px;
  height: 64px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  background: var(--el-color-primary-light-9);
  color: var(--el-color-primary);
  font-size: 28px;
  margin-bottom: 8px;
}

.empty-icon.pulsing {
  animation: pulse 1.6s ease-in-out infinite;
}

@keyframes pulse {
  50% {
    box-shadow: 0 0 0 12px var(--el-color-primary-light-9);
  }
}

.empty-title {
  font-size: 17px;
}

.empty-hints {
  margin: 4px 0 16px;
  padding: 0;
  list-style: none;
  color: var(--el-text-color-secondary);
  display: flex;
  flex-direction: column;
  gap: 4px;
}
</style>

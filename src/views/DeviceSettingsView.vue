<script setup lang="ts">
import { computed } from 'vue';
import { useRoute } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Monitor, SwitchButton, Operation, Picture, Odometer } from '@element-plus/icons-vue';

import { useDevices, isTimesGate } from '../composables/useDevices';
import { invokeCommand } from '../api/times-gate';
import { parseDeviceIp } from '../utils/device';

const route = useRoute();
const { t } = useI18n();
const { findByRouteId, hasScanned } = useDevices();

const routeId = computed(() => route.params.id as string);
const device = computed(() => findByRouteId(routeId.value));
const deviceIp = computed(() => parseDeviceIp(routeId.value));

const title = computed(() => device.value?.name ?? deviceIp.value ?? decodeURIComponent(routeId.value));

// Screens editor is Times Gate specific; show it for unknown devices too until discovery finishes.
const showScreens = computed(() => !hasScanned.value || !device.value || isTimesGate(device.value));

const tabs = computed(() => [
  { key: 'common', label: t('device.tabs.control'), icon: Operation },
  ...(showScreens.value ? [{ key: 'display', label: t('device.tabs.screens'), icon: Picture }] : []),
  { key: 'system', label: t('device.tabs.pcMonitor'), icon: Odometer },
]);

async function handleReboot() {
  if (!deviceIp.value) return;
  try {
    await ElMessageBox.confirm(t('device.rebootConfirm'), t('device.reboot'), {
      confirmButtonText: t('device.reboot'),
      cancelButtonText: t('common.cancel'),
      type: 'warning',
    });
  } catch {
    return;
  }
  await invokeCommand('reboot_device', { ipAddress: deviceIp.value });
  ElMessage.success(t('device.rebootSent'));
}
</script>

<template>
  <div class="app-page device-page">
    <header class="device-hero app-card">
      <div class="hero-icon">
        <el-icon><Monitor /></el-icon>
      </div>
      <div class="hero-text">
        <div class="hero-title-row">
          <h1 class="app-page-title">{{ title }}</h1>
          <el-tag v-if="device" size="small" effect="plain" round>{{ device.device_type }}</el-tag>
        </div>
        <div class="hero-meta">
          <span class="meta-item">
            <span class="app-status-dot" :class="{ 'is-online': device?.is_connected }" />
            {{ device?.is_connected ? t('device.online') : t('device.unknown') }}
          </span>
          <span v-if="deviceIp" class="meta-item app-mono">{{ deviceIp }}</span>
          <span v-if="device?.mac_address" class="meta-item app-mono">{{ device.mac_address }}</span>
        </div>
      </div>
      <div class="hero-actions">
        <el-button :icon="SwitchButton" :disabled="!deviceIp" @click="handleReboot">
          {{ t('device.reboot') }}
        </el-button>
      </div>
    </header>

    <nav class="tabs" role="tablist">
      <router-link v-for="tab in tabs" :key="tab.key" :to="`/device/${routeId}/${tab.key}`" class="tab"
        active-class="is-active" role="tab">
        <el-icon><component :is="tab.icon" /></el-icon>
        {{ tab.label }}
      </router-link>
    </nav>

    <router-view v-slot="{ Component }">
      <component :is="Component" v-if="Component" :key="route.fullPath" />
    </router-view>
  </div>
</template>

<style scoped>
.device-hero {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 18px 20px;
}

.hero-icon {
  width: 52px;
  height: 52px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 14px;
  background: var(--app-screen-bezel);
  color: #fff;
  font-size: 24px;
  box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.08);
}

.hero-text {
  flex: 1;
  min-width: 0;
}

.hero-title-row {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.hero-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 6px 18px;
  margin-top: 4px;
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

.meta-item {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.hero-actions {
  display: flex;
  gap: 8px;
}

.tabs {
  display: flex;
  gap: 4px;
  padding: 4px;
  border-radius: var(--app-radius);
  background: var(--el-fill-color-light);
  border: 1px solid var(--el-border-color-light);
  align-self: flex-start;
}

.tab {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 7px 16px;
  border-radius: 9px;
  color: var(--el-text-color-regular);
  text-decoration: none;
  font-weight: 500;
  transition: background-color 0.15s, color 0.15s;
}

.tab:hover {
  color: var(--el-text-color-primary);
}

.tab.is-active {
  background: var(--el-bg-color);
  color: var(--el-color-primary);
  box-shadow: var(--app-shadow);
}

@media (max-width: 640px) {
  .device-hero {
    flex-wrap: wrap;
  }

  .tabs {
    align-self: stretch;
  }

  .tab {
    flex: 1;
    justify-content: center;
    padding: 7px 8px;
  }
}
</style>

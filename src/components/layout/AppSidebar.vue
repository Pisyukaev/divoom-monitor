<script setup lang="ts">
import { onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { Monitor, Refresh, Setting, Moon, Sunny, House } from '@element-plus/icons-vue';

import { useDevices, getDeviceRouteId } from '../../composables/useDevices';
import { useTheme } from '../../composables/useTheme';
import BrandMark from './BrandMark.vue';

const { t } = useI18n();
const { devices, isScanning, hasScanned, scan, ensureScanned } = useDevices();
const { isDark, toggleTheme } = useTheme();

onMounted(() => {
  ensureScanned();
});
</script>

<template>
  <aside class="sidebar">
    <router-link to="/" class="brand">
      <BrandMark />
      <span class="brand-name">Divoom Monitor</span>
    </router-link>

    <nav class="nav">
      <router-link to="/" class="nav-item" exact-active-class="is-active">
        <el-icon class="nav-icon"><House /></el-icon>
        <span class="nav-label">{{ t('sidebar.overview') }}</span>
      </router-link>

      <div class="section-head">
        <span class="app-section-label nav-label">{{ t('sidebar.devices') }}</span>
        <el-tooltip :content="t('sidebar.rescan')" placement="right">
          <button class="icon-btn" :class="{ spinning: isScanning }" :disabled="isScanning"
            :aria-label="t('sidebar.rescan')" @click="scan">
            <el-icon><Refresh /></el-icon>
          </button>
        </el-tooltip>
      </div>

      <div class="device-list">
        <router-link v-for="device in devices" :key="getDeviceRouteId(device)"
          :to="`/device/${getDeviceRouteId(device)}`" class="nav-item device-item" active-class="is-active">
          <el-icon class="nav-icon"><Monitor /></el-icon>
          <span class="device-text nav-label">
            <span class="device-name">{{ device.name }}</span>
            <span class="device-meta">{{ device.ip_address || device.device_type }}</span>
          </span>
          <span class="app-status-dot" :class="{ 'is-online': device.is_connected }" />
        </router-link>

        <div v-if="devices.length === 0" class="device-empty nav-label">
          <template v-if="isScanning || !hasScanned">{{ t('sidebar.searching') }}</template>
          <template v-else>{{ t('sidebar.noDevices') }}</template>
        </div>
      </div>
    </nav>

    <div class="footer">
      <router-link to="/settings" class="nav-item" active-class="is-active">
        <el-icon class="nav-icon"><Setting /></el-icon>
        <span class="nav-label">{{ t('sidebar.settings') }}</span>
      </router-link>
      <button class="nav-item theme-btn" @click="toggleTheme">
        <el-icon class="nav-icon">
          <Sunny v-if="isDark" />
          <Moon v-else />
        </el-icon>
        <span class="nav-label">{{ isDark ? t('sidebar.lightTheme') : t('sidebar.darkTheme') }}</span>
      </button>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  width: var(--app-sidebar-width);
  flex-shrink: 0;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--app-sidebar-bg);
  border-right: 1px solid var(--el-border-color-light);
  padding: 16px 12px;
  gap: 8px;
}

.brand {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 4px 8px 16px;
  text-decoration: none;
  color: var(--el-text-color-primary);
}

.brand-name {
  font-size: 15px;
  font-weight: 650;
  letter-spacing: -0.01em;
  white-space: nowrap;
}

.nav {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.section-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 8px 6px 12px;
}

.icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--el-text-color-secondary);
  cursor: pointer;
}

.icon-btn:hover:not(:disabled) {
  background: var(--app-hover);
  color: var(--el-text-color-primary);
}

.icon-btn:disabled {
  cursor: default;
}

.icon-btn.spinning .el-icon {
  animation: spin 0.9s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.device-list {
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  min-height: 38px;
  padding: 6px 12px;
  border: none;
  border-radius: var(--app-radius-sm);
  background: transparent;
  color: var(--el-text-color-regular);
  font: inherit;
  text-align: left;
  text-decoration: none;
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s;
}

.nav-item:hover {
  background: var(--app-hover);
  color: var(--el-text-color-primary);
}

.nav-item.is-active {
  background: var(--app-active);
  color: var(--el-color-primary);
  font-weight: 600;
}

.nav-icon {
  font-size: 17px;
  flex-shrink: 0;
}

.device-item {
  min-height: 48px;
}

.device-text {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  line-height: 1.3;
}

.device-name,
.device-meta {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.device-meta {
  font-size: 12px;
  font-weight: 400;
  color: var(--el-text-color-secondary);
  font-family: var(--app-font-mono);
}

.device-empty {
  padding: 6px 12px;
  font-size: 13px;
  color: var(--el-text-color-secondary);
}

.footer {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding-top: 8px;
  border-top: 1px solid var(--el-border-color-light);
}

@media (max-width: 860px) {
  .sidebar {
    width: 68px;
    padding: 16px 10px;
  }

  .nav-label,
  .brand-name {
    display: none;
  }

  .section-head {
    justify-content: center;
    padding: 12px 0 6px;
  }

  .nav-item {
    justify-content: center;
    padding: 6px;
  }

  .brand {
    justify-content: center;
    padding: 4px 0 12px;
  }
}
</style>

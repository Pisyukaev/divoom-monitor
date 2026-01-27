<script setup lang="ts">
import { ref, onMounted, computed, watch, onUnmounted, provide } from 'vue';
import { useRouter, useRoute } from 'vue-router';
import { ArrowLeft, Setting, Monitor, Fold, Expand } from '@element-plus/icons-vue';

import { useDevice } from '../composables/useDevice';
import { scanDevices } from '../api/common';
import type { DivoomDevice } from '../types/device';

const router = useRouter();
const route = useRoute();
const { fetchDeviceSettings } = useDevice();

const deviceId = computed(() => route.params.id as string);
const deviceInfo = ref<DivoomDevice | null>(null);
const isLoadingDevice = ref(false);
const sidebarWidth = ref(250);
const isCollapsed = ref(false);
const isResizing = ref(false);

provide('deviceInfo', deviceInfo);

const MIN_SIDEBAR_WIDTH = 200;
const MAX_SIDEBAR_WIDTH = 500;
const COLLAPSED_WIDTH = 60;

const isTimesGate = computed(() => {
  return deviceInfo.value?.device_type === 'Times Gate';
});

const deviceIp = computed(() => {
  const decodedId = decodeURIComponent(deviceId.value);
  if (decodedId.match(/^\d+\.\d+\.\d+\.\d+$/)) {
    return decodedId;
  }
  return deviceInfo.value?.ip_address || '';
});

const activeMenu = computed(() => {
  const path = route.path;
  if (path.includes('/common')) {
    return 'common';
  } else if (path.includes('/display')) {
    return 'display';
  }
  return 'common';
});

async function loadDeviceInfo() {
  try {
    isLoadingDevice.value = true;
    const devices = await scanDevices();
    const decodedId = decodeURIComponent(deviceId.value);
    const foundDevice = devices.find(
      (d) =>
        d.ip_address === decodedId ||
        d.mac_address === decodedId ||
        d.name === decodedId
    );
    if (foundDevice) {
      deviceInfo.value = foundDevice;
    }
  } catch (error) {
    console.error('Error loading device info:', error);
  } finally {
    isLoadingDevice.value = false;
  }
}

function handleMenuSelect(key: string) {
  router.push(key);
}

function handleUpdateSettings() {
  fetchDeviceSettings(deviceId.value);
}

function goBack() {
  router.push('/');
}

function toggleSidebar() {
  isCollapsed.value = !isCollapsed.value;
  if (isCollapsed.value) {
    if (sidebarWidth.value > COLLAPSED_WIDTH) {
      localStorage.setItem('sidebarWidth', sidebarWidth.value.toString());
    }
  } else {
    const savedWidth = localStorage.getItem('sidebarWidth');
    if (savedWidth) {
      sidebarWidth.value = Math.max(MIN_SIDEBAR_WIDTH, Math.min(MAX_SIDEBAR_WIDTH, parseInt(savedWidth)));
    }
  }
}

function startResize(e: MouseEvent) {
  isResizing.value = true;
  e.preventDefault();
}

function handleResize(e: MouseEvent) {
  if (!isResizing.value) return;

  const newWidth = e.clientX;
  if (newWidth >= MIN_SIDEBAR_WIDTH && newWidth <= MAX_SIDEBAR_WIDTH) {
    sidebarWidth.value = newWidth;
    localStorage.setItem('sidebarWidth', newWidth.toString());
  }
}

function stopResize() {
  isResizing.value = false;
}

onMounted(() => {
  const savedWidth = localStorage.getItem('sidebarWidth');
  if (savedWidth) {
    sidebarWidth.value = Math.max(MIN_SIDEBAR_WIDTH, Math.min(MAX_SIDEBAR_WIDTH, parseInt(savedWidth)));
  }

  // Загружаем данные при монтировании
  if (route.params.id) {
    handleUpdateSettings();
    loadDeviceInfo();
  }

  document.addEventListener('mousemove', handleResize);
  document.addEventListener('mouseup', stopResize);
});

onUnmounted(() => {
  document.removeEventListener('mousemove', handleResize);
  document.removeEventListener('mouseup', stopResize);
});

// Отслеживаем только изменение deviceId, а не route.name
// Это предотвращает повторные запросы при переключении между роутами одного устройства
watch(
  () => route.params.id,
  (newId: string | string[], oldId: string | string[] | undefined) => {
    // Загружаем данные только если deviceId действительно изменился
    if (newId && newId !== oldId) {
      handleUpdateSettings();
      loadDeviceInfo();
    }
  }
);
</script>

<template>
  <div class="device-settings-container">
    <!-- Боковая панель -->
    <aside class="sidebar" :class="{ collapsed: isCollapsed }"
      :style="{ width: isCollapsed ? `${COLLAPSED_WIDTH}px` : `${sidebarWidth}px` }">
      <div class="sidebar-header">
        <h3 v-if="!isCollapsed">{{ deviceInfo ? deviceInfo.name : 'Настройки' }}</h3>
        <el-button :icon="isCollapsed ? Expand : Fold" @click="toggleSidebar" circle size="small"
          class="collapse-button" />
      </div>

      <el-menu :default-active="activeMenu" @select="handleMenuSelect" class="settings-menu" :collapse="isCollapsed">
        <el-menu-item index="common">
          <el-icon>
            <Setting />
          </el-icon>
          <template #title>
            <span>Общие настройки</span>
          </template>
        </el-menu-item>
        <el-menu-item v-if="isTimesGate && deviceIp" index="display">
          <el-icon>
            <Monitor />
          </el-icon>
          <template #title>
            <span>Настройки экранов</span>
          </template>
        </el-menu-item>
      </el-menu>

      <!-- Разделитель для ресайза -->
      <div v-if="!isCollapsed" class="resize-handle" @mousedown="startResize" :class="{ resizing: isResizing }"></div>
    </aside>

    <!-- Основной контент -->
    <main class="main-content" :style="{ left: isCollapsed ? `${COLLAPSED_WIDTH}px` : `${sidebarWidth}px` }">
      <div class="content-header">
        <el-button :icon="ArrowLeft" @click="goBack" circle class="back-button" />
        <h2>{{ deviceInfo ? deviceInfo.name : 'Настройки устройства' }}</h2>
      </div>

      <div class="content-area">
        <slot />
      </div>
    </main>
  </div>
</template>

<style scoped>
.device-settings-container {
  display: flex;
  height: calc(100vh - 80px);
  overflow: hidden;
  position: relative;
  width: 100%;
  background-color: var(--el-bg-color-page);
}

.sidebar {
  position: fixed;
  left: 0;
  top: 80px;
  height: calc(100vh - 80px);
  background-color: var(--el-bg-color);
  border-right: 1px solid var(--el-border-color);
  display: flex;
  flex-direction: column;
  transition: width 0.3s ease;
  z-index: 1000;
  box-shadow: 2px 0 8px rgba(0, 0, 0, 0.1);
}

.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px;
  border-bottom: 1px solid var(--el-border-color);
}

.sidebar-header h3 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: var(--el-text-color-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.collapse-button {
  flex-shrink: 0;
}

.sidebar.collapsed .sidebar-header {
  justify-content: center;
}

.settings-menu {
  flex: 1;
  border-right: none;
  overflow-y: auto;
}

.resize-handle {
  position: absolute;
  right: 0;
  top: 0;
  width: 4px;
  height: 100%;
  cursor: col-resize;
  background-color: transparent;
  transition: background-color 0.2s;
  z-index: 10;
}

.resize-handle:hover {
  background-color: var(--el-color-primary);
}

.resize-handle.resizing {
  background-color: var(--el-color-primary);
}

.main-content {
  position: fixed;
  top: 80px;
  right: 0;
  bottom: 0;
  left: 0;
  display: flex;
  flex-direction: column;
  transition: left 0.3s ease;
  overflow: hidden;
  background-color: var(--el-bg-color-page);
}

.content-header {
  display: flex;
  align-items: center;
  gap: 15px;
  padding: 20px;
  background-color: var(--el-bg-color);
  border-bottom: 1px solid var(--el-border-color);
  flex-shrink: 0;
}

.back-button {
  flex-shrink: 0;
}

.content-header h2 {
  margin: 0;
  font-size: 24px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.content-area {
  flex: 1;
  padding: 20px;
  overflow-y: auto;
}

:deep(.el-menu-item) {
  height: 50px;
  line-height: 50px;
}

:deep(.el-menu-item span) {
  margin-left: 8px;
}

:deep(.el-menu--collapse .el-menu-item span) {
  display: none;
}
</style>

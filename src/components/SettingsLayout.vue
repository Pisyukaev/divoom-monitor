<script setup lang="ts">
import { ref, onMounted, computed, onUnmounted, provide } from 'vue';
import { useRouter, useRoute } from 'vue-router';
import { ArrowLeft, Setting, Monitor, Fold, Expand, Odometer } from '@element-plus/icons-vue';

import { useDevice } from '../composables/useDevice';
import ThemeToggle from './ThemeToggle.vue';

const router = useRouter();
const route = useRoute();
const { settings } = useDevice();

const DEFAULT_SIDEBAR_WIDTH = 250
const MIN_SIDEBAR_WIDTH = 200;
const MAX_SIDEBAR_WIDTH = 500;
const COLLAPSED_WIDTH = 60;

const sidebarWidth = ref(DEFAULT_SIDEBAR_WIDTH);
const isCollapsed = ref(false);
const isResizing = ref(false);

provide('settings', settings)


const activeMenu = computed(() => {
  const path = route.path;
  if (path.includes('/common')) {
    return 'common';
  } else if (path.includes('/display')) {
    return 'display';
  } else if (path.includes('/system')) {
    return 'system';
  }
  return 'common';
});

function handleMenuSelect(key: string) {
  // Используем текущий путь устройства и добавляем нужный подпуть
  const deviceId = route.params.id;
  router.push(`/device/${deviceId}/${key}`);
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

function startResize() {
  isResizing.value = true;
}

function handleResize(e: MouseEvent) {
  if (!isResizing.value) {
    return;
  }

  const newWidth = e.clientX;
  if (newWidth >= MIN_SIDEBAR_WIDTH && newWidth <= MAX_SIDEBAR_WIDTH) {
    sidebarWidth.value = newWidth;
  }
}

function stopResize() {
  if (!isResizing.value) {
    return;
  }

  isResizing.value = false;
  localStorage.setItem('sidebarWidth', sidebarWidth.value.toString());
}

onMounted(() => {
  const savedWidth = localStorage.getItem('sidebarWidth');
  if (savedWidth) {
    sidebarWidth.value = Math.max(MIN_SIDEBAR_WIDTH, Math.min(MAX_SIDEBAR_WIDTH, parseInt(savedWidth)));
  }


  document.addEventListener('mousemove', handleResize);
  document.addEventListener('mouseup', stopResize);
});

onUnmounted(() => {
  document.removeEventListener('mousemove', handleResize);
  document.removeEventListener('mouseup', stopResize);
});
</script>

<template>
  <div class="device-settings-container">
    <!-- Боковая панель -->
    <aside class="sidebar" :class="{ collapsed: isCollapsed, resizing: isResizing }"
      :style="{ width: isCollapsed ? `${COLLAPSED_WIDTH}px` : `${sidebarWidth}px` }">
      <div class="sidebar-header">
        <h3 v-if="!isCollapsed">{{ 'Настройки' }}</h3>
        <el-button :icon="isCollapsed ? Expand : Fold" @click="toggleSidebar" circle size="small"
          class="collapse-button" />
      </div>

      <el-menu :default-active="activeMenu" @select="handleMenuSelect" class="settings-menu" :collapse="isCollapsed"
        :collapse-transition="false">
        <el-menu-item index="common">
          <el-icon>
            <Setting />
          </el-icon>
          <template #title>
            <span>Общие настройки</span>
          </template>
        </el-menu-item>
        <el-menu-item index="display">
          <el-icon>
            <Monitor />
          </el-icon>
          <template #title>
            <span>Настройки экранов</span>
          </template>
        </el-menu-item>
        <el-menu-item index="system">
          <el-icon>
            <Odometer />
          </el-icon>
          <template #title>
            <span>Состояние системы</span>
          </template>
        </el-menu-item>
      </el-menu>

      <!-- Разделитель для ресайза -->
      <div v-if="!isCollapsed" class="resize-handle" @mousedown="startResize" :class="{ resizing: isResizing }"></div>
    </aside>

    <!-- Основной контент -->
    <main class="main-content" :class="{ resizing: isResizing }"
      :style="{ left: isCollapsed ? `${COLLAPSED_WIDTH}px` : `${sidebarWidth}px` }">
      <div class="content-header">
        <el-button :icon="ArrowLeft" @click="goBack" circle class="back-button" />
        <h2>{{ 'Настройки устройства' }}</h2>
        <ThemeToggle />
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
  top: 0;
  height: 100vh;
  background-color: var(--el-bg-color);
  border-right: 1px solid var(--el-border-color);
  display: flex;
  flex-direction: column;
  z-index: 1000;
  box-shadow: 2px 0 8px rgba(0, 0, 0, 0.1);
}

.resizing {
  cursor: col-resize;
  user-select: none;
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
  top: 0;
  right: 0;
  bottom: 0;
  left: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background-color: var(--el-bg-color-page);
}

.content-header {
  display: flex;
  align-items: center;
  gap: 15px;
  padding-left: 20px;
  background-color: var(--el-bg-color);
  border-bottom: 1px solid var(--el-border-color);
  flex-shrink: 0;
}

.back-button {
  flex-shrink: 0;
}

.content-header h2 {
  color: var(--el-text-color-primary);
}

.content-area {
  flex: 1;
  padding: 20px;
  overflow-y: auto;
}
</style>

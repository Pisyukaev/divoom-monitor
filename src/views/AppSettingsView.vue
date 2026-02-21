<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { useRouter } from 'vue-router';
import { invoke } from '@tauri-apps/api/core';
import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart';
import { ArrowLeft, Moon, Sunny } from '@element-plus/icons-vue';
import { useTheme } from '../composables/useTheme';

const router = useRouter();
const { isDark, toggleTheme } = useTheme();

const autoStartEnabled = ref(false);
const autoStartLoading = ref(false);
const closeToTray = ref(true);
const closeToTrayLoading = ref(false);

const themeLabel = computed(() => isDark.value ? 'Тёмная' : 'Светлая');

onMounted(async () => {
  try {
    autoStartEnabled.value = await isEnabled();
  } catch (err) {
    console.error('Failed to check autostart status:', err);
  }

  try {
    closeToTray.value = await invoke<boolean>('get_close_to_tray');
  } catch (err) {
    console.error('Failed to get close-to-tray setting:', err);
  }
});

async function handleAutoStartChange(value: boolean) {
  autoStartLoading.value = true;
  try {
    if (value) {
      await enable();
    } else {
      await disable();
    }
    autoStartEnabled.value = await isEnabled();
  } catch (err) {
    console.error('Failed to toggle autostart:', err);
    autoStartEnabled.value = !value;
  } finally {
    autoStartLoading.value = false;
  }
}

async function handleCloseToTrayChange(value: boolean) {
  closeToTrayLoading.value = true;
  try {
    await invoke('set_close_to_tray', { enabled: value });
    closeToTray.value = value;
  } catch (err) {
    console.error('Failed to toggle close-to-tray:', err);
    closeToTray.value = !value;
  } finally {
    closeToTrayLoading.value = false;
  }
}

function goBack() {
  router.push('/');
}
</script>

<template>
  <div class="app-settings-page">
    <header class="settings-header">
      <el-button :icon="ArrowLeft" @click="goBack" circle class="back-button" />
      <h2>Настройки приложения</h2>
    </header>

    <div class="settings-content">
      <el-card class="settings-card" shadow="hover">
        <template #header>
          <span class="card-title">Внешний вид</span>
        </template>

        <el-descriptions :column="1" border>
          <el-descriptions-item label="Тема приложения">
            <div class="setting-row">
              <span class="setting-value-label">{{ themeLabel }}</span>
              <el-button :icon="isDark ? Sunny : Moon" circle @click="toggleTheme"
                :title="isDark ? 'Переключить на светлую тему' : 'Переключить на тёмную тему'" />
            </div>
          </el-descriptions-item>
        </el-descriptions>
      </el-card>

      <el-card class="settings-card" shadow="hover">
        <template #header>
          <span class="card-title">Поведение</span>
        </template>

        <el-descriptions :column="1" border>
          <el-descriptions-item label="Автозапуск при старте Windows">
            <div class="setting-row">
              <span class="setting-description">Приложение запустится автоматически при входе в систему</span>
              <el-switch v-model="autoStartEnabled" :loading="autoStartLoading"
                @change="(val: string | number | boolean) => handleAutoStartChange(Boolean(val))" />
            </div>
          </el-descriptions-item>

          <el-descriptions-item label="Сворачивать в трей при закрытии">
            <div class="setting-row">
              <span class="setting-description">Приложение свернётся в системный трей вместо полного закрытия</span>
              <el-switch v-model="closeToTray" :loading="closeToTrayLoading"
                @change="(val: string | number | boolean) => handleCloseToTrayChange(Boolean(val))" />
            </div>
          </el-descriptions-item>
        </el-descriptions>
      </el-card>
    </div>
  </div>
</template>

<style scoped>
.app-settings-page {
  display: flex;
  flex-direction: column;
  min-height: 100vh;
  background-color: var(--el-bg-color-page);
}

.settings-header {
  display: flex;
  align-items: center;
  gap: 15px;
  padding: 16px 20px;
  background-color: var(--el-bg-color);
  border-bottom: 1px solid var(--el-border-color);
  flex-shrink: 0;
}

.settings-header h2 {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.back-button {
  flex-shrink: 0;
}

.settings-content {
  max-width: 800px;
  width: 100%;
  margin: 0 auto;
  padding: 24px 20px;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.settings-card {
  width: 100%;
}

.card-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  width: 100%;
}

.setting-value-label {
  font-weight: 500;
  color: var(--el-text-color-primary);
}

.setting-description {
  font-size: 13px;
  color: var(--el-text-color-secondary);
  flex: 1;
}

@media (max-width: 768px) {
  .settings-content {
    padding: 16px 12px;
  }

  .setting-row {
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
  }
}
</style>

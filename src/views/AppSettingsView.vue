<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';
import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { ArrowLeft, Moon, Sunny, Refresh } from '@element-plus/icons-vue';
import { useTheme } from '../composables/useTheme';
import { saveLocale, type Locale } from '../i18n';

const router = useRouter();
const { t, locale } = useI18n();
const { isDark, toggleTheme } = useTheme();

const autoStartEnabled = ref(false);
const autoStartLoading = ref(false);
const closeToTray = ref(true);
const closeToTrayLoading = ref(false);

const appVersion = ref('');
const updateChecking = ref(false);
const updateAvailable = ref<Update | null>(null);
const updateDownloading = ref(false);
const updateDownloaded = ref(0);
const updateTotal = ref(0);
const updateStatus = ref<'idle' | 'checking' | 'available' | 'no-update' | 'downloading' | 'ready' | 'error'>('idle');
const updateError = ref('');

const themeLabel = computed(() => isDark.value ? t('appSettings.dark') : t('appSettings.light'));

const downloadPercent = computed(() => {
  if (updateTotal.value === 0) return 0;
  return Math.round((updateDownloaded.value / updateTotal.value) * 100);
});

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

async function checkForUpdates() {
  updateChecking.value = true;
  updateStatus.value = 'checking';
  updateError.value = '';
  updateAvailable.value = null;

  try {
    const update = await check();
    if (update) {
      updateAvailable.value = update;
      updateStatus.value = 'available';
    } else {
      updateStatus.value = 'no-update';
    }
  } catch (err) {
    console.error('Failed to check for updates:', err);
    updateStatus.value = 'error';
    updateError.value = String(err);
  } finally {
    updateChecking.value = false;
  }
}

async function downloadAndInstall() {
  if (!updateAvailable.value) return;

  updateDownloading.value = true;
  updateStatus.value = 'downloading';
  updateDownloaded.value = 0;
  updateTotal.value = 0;

  try {
    await updateAvailable.value.downloadAndInstall((event) => {
      switch (event.event) {
        case 'Started':
          updateTotal.value = event.data.contentLength ?? 0;
          break;
        case 'Progress':
          updateDownloaded.value += event.data.chunkLength;
          break;
        case 'Finished':
          updateStatus.value = 'ready';
          break;
      }
    });
    await relaunch();
  } catch (err) {
    console.error('Failed to download/install update:', err);
    updateStatus.value = 'error';
    updateError.value = String(err);
  } finally {
    updateDownloading.value = false;
  }
}

const languageOptions = [
  { value: 'ru', label: 'Русский' },
  { value: 'en', label: 'English' },
];

function handleLocaleChange(value: string) {
  locale.value = value;
  saveLocale(value as Locale);
}

onMounted(async () => {
  try {
    appVersion.value = await getVersion();
  } catch (err) {
    console.error('Failed to get app version:', err);
  }

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
      <h2>{{ t('appSettings.title') }}</h2>
    </header>

    <div class="settings-content">
      <el-card class="settings-card" shadow="hover">
        <template #header>
          <span class="card-title">{{ t('appSettings.appearance') }}</span>
        </template>

        <el-descriptions :column="1" border>
          <el-descriptions-item :label="t('appSettings.appTheme')">
            <div class="setting-row">
              <span class="setting-value-label">{{ themeLabel }}</span>
              <el-button :icon="isDark ? Sunny : Moon" circle @click="toggleTheme"
                :title="isDark ? t('appSettings.switchToLight') : t('appSettings.switchToDark')" />
            </div>
          </el-descriptions-item>

          <el-descriptions-item :label="t('appSettings.languageLabel')">
            <div class="setting-row">
              <el-select :model-value="locale" @change="handleLocaleChange" style="width: 200px">
                <el-option v-for="opt in languageOptions" :key="opt.value" :label="opt.label" :value="opt.value" />
              </el-select>
            </div>
          </el-descriptions-item>
        </el-descriptions>
      </el-card>

      <el-card class="settings-card" shadow="hover">
        <template #header>
          <span class="card-title">{{ t('appSettings.behavior') }}</span>
        </template>

        <el-descriptions :column="1" border>
          <el-descriptions-item :label="t('appSettings.autostart')">
            <div class="setting-row">
              <span class="setting-description">{{ t('appSettings.autostartDescription') }}</span>
              <el-switch v-model="autoStartEnabled" :loading="autoStartLoading"
                @change="(val: string | number | boolean) => handleAutoStartChange(Boolean(val))" />
            </div>
          </el-descriptions-item>

          <el-descriptions-item :label="t('appSettings.closeToTray')">
            <div class="setting-row">
              <span class="setting-description">{{ t('appSettings.closeToTrayDescription') }}</span>
              <el-switch v-model="closeToTray" :loading="closeToTrayLoading"
                @change="(val: string | number | boolean) => handleCloseToTrayChange(Boolean(val))" />
            </div>
          </el-descriptions-item>
        </el-descriptions>
      </el-card>

      <el-card class="settings-card" shadow="hover">
        <template #header>
          <span class="card-title">{{ t('appSettings.updates') }}</span>
        </template>

        <el-descriptions :column="1" border>
          <el-descriptions-item :label="t('appSettings.currentVersion')">
            <div class="setting-row">
              <span class="setting-value-label">v{{ appVersion }}</span>
              <el-button
                :icon="Refresh"
                :loading="updateChecking"
                @click="checkForUpdates"
                :disabled="updateDownloading"
              >
                {{ updateChecking ? t('appSettings.checking') : t('appSettings.checkForUpdates') }}
              </el-button>
            </div>
          </el-descriptions-item>
        </el-descriptions>

        <div v-if="updateStatus === 'no-update'" class="update-status update-status--success">
          {{ t('appSettings.noUpdates') }}
        </div>

        <div v-if="updateStatus === 'error'" class="update-status update-status--error">
          {{ t('appSettings.updateError') }}: {{ updateError }}
        </div>

        <div v-if="updateAvailable && (updateStatus === 'available' || updateStatus === 'downloading' || updateStatus === 'ready')" class="update-info">
          <el-alert
            :title="t('appSettings.updateAvailable', { version: updateAvailable.version })"
            type="success"
            :closable="false"
            show-icon
          />

          <div v-if="updateAvailable.body" class="release-notes">
            <span class="release-notes-label">{{ t('appSettings.releaseNotes') }}:</span>
            <p class="release-notes-body">{{ updateAvailable.body }}</p>
          </div>

          <div v-if="updateStatus === 'downloading'" class="download-progress">
            <span class="download-label">{{ t('appSettings.downloading') }}</span>
            <el-progress :percentage="downloadPercent" :stroke-width="18" striped striped-flow />
            <span v-if="updateTotal > 0" class="download-detail">
              {{ formatBytes(updateDownloaded) }} / {{ formatBytes(updateTotal) }}
            </span>
          </div>

          <el-button
            v-if="updateStatus === 'available'"
            type="primary"
            @click="downloadAndInstall"
            :loading="updateDownloading"
            class="install-button"
          >
            {{ t('appSettings.installAndRestart') }}
          </el-button>
        </div>
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

.update-status {
  margin-top: 16px;
  padding: 10px 16px;
  border-radius: 6px;
  font-size: 14px;
}

.update-status--success {
  background-color: var(--el-color-success-light-9);
  color: var(--el-color-success);
}

.update-status--error {
  background-color: var(--el-color-danger-light-9);
  color: var(--el-color-danger);
}

.update-info {
  margin-top: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.release-notes {
  padding: 12px;
  background-color: var(--el-fill-color-light);
  border-radius: 6px;
}

.release-notes-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.release-notes-body {
  margin: 6px 0 0;
  font-size: 13px;
  color: var(--el-text-color-secondary);
  white-space: pre-wrap;
}

.download-progress {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.download-label {
  font-size: 13px;
  color: var(--el-text-color-secondary);
}

.download-detail {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  text-align: right;
}

.install-button {
  align-self: flex-start;
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

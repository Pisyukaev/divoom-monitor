<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';
import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { Refresh, Brush, ChatDotRound, Promotion, Bottom, Download } from '@element-plus/icons-vue';

import SettingRow from '../components/ui/SettingRow.vue';
import { useTheme } from '../composables/useTheme';
import { saveLocale, type Locale } from '../i18n';

const { t, locale } = useI18n();
const { isDark, setTheme } = useTheme();

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

const themeOptions = computed(() => [
  { value: 'light', label: t('appSettings.light') },
  { value: 'dark', label: t('appSettings.dark') },
]);

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
</script>

<template>
  <div class="app-page settings-page">
    <header>
      <h1 class="app-page-title">{{ t('appSettings.title') }}</h1>
      <p class="app-page-subtitle">{{ t('appSettings.subtitle') }}</p>
    </header>

    <section class="app-card">
      <div class="app-card-header">
        <span class="app-card-title">{{ t('appSettings.appearance') }}</span>
      </div>
      <div class="app-card-body">
        <SettingRow :icon="Brush" :title="t('appSettings.appTheme')">
          <el-segmented :model-value="isDark ? 'dark' : 'light'" :options="themeOptions"
            @change="(v: 'light' | 'dark') => setTheme(v)" />
        </SettingRow>
        <SettingRow :icon="ChatDotRound" :title="t('appSettings.languageLabel')">
          <el-select :model-value="locale" style="width: 160px" @change="handleLocaleChange">
            <el-option v-for="opt in languageOptions" :key="opt.value" :label="opt.label" :value="opt.value" />
          </el-select>
        </SettingRow>
      </div>
    </section>

    <section class="app-card">
      <div class="app-card-header">
        <span class="app-card-title">{{ t('appSettings.behavior') }}</span>
      </div>
      <div class="app-card-body">
        <SettingRow :icon="Promotion" :title="t('appSettings.autostart')"
          :description="t('appSettings.autostartDescription')">
          <el-switch v-model="autoStartEnabled" :loading="autoStartLoading"
            @change="(val: string | number | boolean) => handleAutoStartChange(Boolean(val))" />
        </SettingRow>
        <SettingRow :icon="Bottom" :title="t('appSettings.closeToTray')"
          :description="t('appSettings.closeToTrayDescription')">
          <el-switch v-model="closeToTray" :loading="closeToTrayLoading"
            @change="(val: string | number | boolean) => handleCloseToTrayChange(Boolean(val))" />
        </SettingRow>
      </div>
    </section>

    <section class="app-card">
      <div class="app-card-header">
        <span class="app-card-title">{{ t('appSettings.updates') }}</span>
      </div>
      <div class="app-card-body">
        <SettingRow :icon="Download" :title="t('appSettings.currentVersion')"
          :description="appVersion ? `v${appVersion}` : undefined">
          <el-button :icon="Refresh" :loading="updateChecking" :disabled="updateDownloading" @click="checkForUpdates">
            {{ updateChecking ? t('appSettings.checking') : t('appSettings.checkForUpdates') }}
          </el-button>
        </SettingRow>

        <el-alert v-if="updateStatus === 'no-update'" :title="t('appSettings.noUpdates')" type="success"
          :closable="false" show-icon class="update-alert" />
        <el-alert v-if="updateStatus === 'error'" :title="t('appSettings.updateError')" :description="updateError"
          type="error" :closable="false" show-icon class="update-alert" />

        <div
          v-if="updateAvailable && (updateStatus === 'available' || updateStatus === 'downloading' || updateStatus === 'ready')"
          class="update-info">
          <el-alert :title="t('appSettings.updateAvailable', { version: updateAvailable.version })" type="info"
            :closable="false" show-icon />

          <div v-if="updateAvailable.body" class="release-notes">
            <div class="app-section-label">{{ t('appSettings.releaseNotes') }}</div>
            <p class="release-notes-body">{{ updateAvailable.body }}</p>
          </div>

          <div v-if="updateStatus === 'downloading'" class="download-progress">
            <span class="app-muted">{{ t('appSettings.downloading') }}</span>
            <el-progress :percentage="downloadPercent" :stroke-width="10" />
            <span v-if="updateTotal > 0" class="app-muted download-detail">
              {{ formatBytes(updateDownloaded) }} / {{ formatBytes(updateTotal) }}
            </span>
          </div>

          <el-button v-if="updateStatus === 'available'" type="primary" :loading="updateDownloading"
            class="install-button" @click="downloadAndInstall">
            {{ t('appSettings.installAndRestart') }}
          </el-button>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.settings-page {
  max-width: 760px;
}

.update-alert {
  margin-top: 8px;
}

.update-info {
  margin-top: 8px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.release-notes {
  padding: 12px 14px;
  background-color: var(--el-fill-color-light);
  border-radius: var(--app-radius-sm);
}

.release-notes-body {
  margin: 6px 0 0;
  font-size: 13px;
  color: var(--el-text-color-regular);
  white-space: pre-wrap;
}

.download-progress {
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: 13px;
}

.download-detail {
  text-align: right;
  font-size: 12px;
}

.install-button {
  align-self: flex-start;
}
</style>

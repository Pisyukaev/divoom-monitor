<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { useRouter, useRoute } from 'vue-router';
import { ArrowLeft, Refresh } from '@element-plus/icons-vue';

import { useDevice } from '../composables/useDevice';
import type { DivoomDevice } from '../types/device';

const router = useRouter();
const route = useRoute();
const {
  settings,
  isLoadingSettings,
  settingsError,
  fetchDeviceById,
  fetchDeviceSettings,
} = useDevice();

const deviceId = computed(() => route.params.id as string);
const deviceInfo = ref<DivoomDevice | null>(null);
const isLoadingDevice = ref(false);
const deviceError = ref<string | null>(null);

async function loadDevice() {
  isLoadingDevice.value = true;
  deviceError.value = null;

  try {
    const loadedDevice = await fetchDeviceById(deviceId.value);
    if (loadedDevice) {
      deviceInfo.value = loadedDevice;
      if (loadedDevice.ip_address) {
        await fetchDeviceSettings(loadedDevice.ip_address);
      }
    } else {
      deviceError.value = 'Устройство не найдено';
    }
  } catch (error) {
    deviceError.value =
      error instanceof Error ? error.message : 'Ошибка загрузки устройства';
    console.error('Error loading device:', error);
  } finally {
    isLoadingDevice.value = false;
  }
}

async function handleRefreshSettings() {
  if (deviceInfo.value?.ip_address) {
    await fetchDeviceSettings(deviceInfo.value.ip_address);
  }
}

function goBack() {
  router.push('/');
}

onMounted(() => {
  loadDevice();
});
</script>

<template>
  <div class="device-settings-container">
    <div class="header-section">
      <el-button :icon="ArrowLeft" @click="goBack" circle class="back-button" />
      <h2 v-if="deviceInfo">{{ deviceInfo.name }}</h2>
      <h2 v-else>Настройки устройства</h2>
    </div>

    <el-loading
      v-if="isLoadingDevice"
      :loading="isLoadingDevice"
      text="Загрузка устройства..."
    />

    <el-alert
      v-if="deviceError"
      :title="deviceError"
      type="error"
      :closable="false"
      show-icon
      style="margin-bottom: 20px"
    />

    <div v-if="deviceInfo && !isLoadingDevice" class="content-section">
      <!-- Информация об устройстве -->
      <el-card class="info-card" shadow="hover">
        <template #header>
          <div class="card-header">
            <span>Информация об устройстве</span>
          </div>
        </template>
        <el-descriptions :column="1" border>
          <el-descriptions-item label="Тип устройства">
            {{ deviceInfo.device_type }}
          </el-descriptions-item>
          <el-descriptions-item v-if="deviceInfo.model" label="Модель">
            {{ deviceInfo.model }}
          </el-descriptions-item>
          <el-descriptions-item v-if="deviceInfo.ip_address" label="IP адрес">
            {{ deviceInfo.ip_address }}
          </el-descriptions-item>
          <el-descriptions-item v-if="deviceInfo.mac_address" label="MAC адрес">
            {{ deviceInfo.mac_address }}
          </el-descriptions-item>
          <el-descriptions-item label="Статус">
            <el-tag :type="deviceInfo.is_connected ? 'success' : 'danger'">
              {{ deviceInfo.is_connected ? 'Подключено' : 'Не подключено' }}
            </el-tag>
          </el-descriptions-item>
          <el-descriptions-item
            v-if="deviceInfo.signal_strength !== null"
            label="Уровень сигнала"
          >
            {{ deviceInfo.signal_strength }} dBm
          </el-descriptions-item>
        </el-descriptions>
      </el-card>

      <!-- Текущие настройки -->
      <el-card class="settings-card" shadow="hover">
        <template #header>
          <div class="card-header">
            <span>Текущие настройки</span>
            <el-button
              :icon="Refresh"
              @click="handleRefreshSettings"
              :loading="isLoadingSettings"
              size="small"
              circle
              :title="'Обновить настройки'"
            />
          </div>
        </template>

        <el-loading
          v-if="isLoadingSettings"
          :loading="isLoadingSettings"
          text="Загрузка настроек..."
        />

        <el-alert
          v-if="settingsError"
          :title="settingsError"
          type="error"
          :closable="false"
          show-icon
          style="margin-bottom: 20px"
        />

        <div v-if="settings && !isLoadingSettings">
          <!-- Основные настройки -->
          <el-descriptions title="Основные настройки" :column="1" border>
            <el-descriptions-item
              v-if="settings.brightness !== undefined"
              label="Яркость"
            >
              <div class="setting-value">
                <el-progress
                  :percentage="settings.brightness"
                  :format="(percentage) => `${percentage}%`"
                />
              </div>
            </el-descriptions-item>
            <el-descriptions-item
              v-if="settings.volume !== undefined"
              label="Громкость"
            >
              <div class="setting-value">
                <el-progress
                  :percentage="settings.volume"
                  :format="(percentage) => `${percentage}%`"
                />
              </div>
            </el-descriptions-item>
            <el-descriptions-item
              v-if="settings.display_mode"
              label="Режим отображения"
            >
              <el-tag>{{ settings.display_mode }}</el-tag>
            </el-descriptions-item>
            <el-descriptions-item
              v-if="settings.current_time"
              label="Текущее время"
            >
              {{ settings.current_time }}
            </el-descriptions-item>
          </el-descriptions>

          <!-- Сетевые настройки -->
          <el-descriptions
            v-if="settings.network_settings"
            title="Сетевые настройки"
            :column="1"
            border
            style="margin-top: 20px"
          >
            <el-descriptions-item
              v-if="settings.network_settings.ssid"
              label="SSID"
            >
              {{ settings.network_settings.ssid }}
            </el-descriptions-item>
            <el-descriptions-item
              v-if="settings.network_settings.ip_address"
              label="IP адрес"
            >
              {{ settings.network_settings.ip_address }}
            </el-descriptions-item>
            <el-descriptions-item
              v-if="settings.network_settings.mac_address"
              label="MAC адрес"
            >
              {{ settings.network_settings.mac_address }}
            </el-descriptions-item>
            <el-descriptions-item
              v-if="settings.network_settings.signal_strength !== undefined"
              label="Уровень сигнала"
            >
              {{ settings.network_settings.signal_strength }} dBm
            </el-descriptions-item>
          </el-descriptions>

          <!-- Дополнительные настройки -->
          <el-descriptions
            title="Дополнительные настройки"
            :column="1"
            border
            style="margin-top: 20px"
          >
            <el-descriptions-item
              v-for="(value, key) in settings"
              :key="key"
              v-if="
                key !== 'brightness' &&
                key !== 'volume' &&
                key !== 'display_mode' &&
                key !== 'current_time' &&
                key !== 'network_settings' &&
                value !== null &&
                value !== undefined
              "
              :label="key"
            >
              <el-tag v-if="typeof value === 'boolean'">
                {{ value ? 'Включено' : 'Выключено' }}
              </el-tag>
              <span v-else>{{ String(value) }}</span>
            </el-descriptions-item>
          </el-descriptions>
        </div>

        <el-empty
          v-if="!settings && !isLoadingSettings && !settingsError"
          description="Настройки не загружены. Нажмите 'Обновить' для загрузки."
        />
      </el-card>

      <!-- Секции для будущего расширения -->
      <el-card class="actions-card" shadow="hover" style="margin-top: 20px">
        <template #header>
          <div class="card-header">
            <span>Управление устройством</span>
          </div>
        </template>
        <el-alert
          title="Функции управления будут добавлены в будущих версиях"
          type="info"
          :closable="false"
          show-icon
        />
      </el-card>
    </div>
  </div>
</template>

<style scoped>
.device-settings-container {
  max-width: 1200px;
  margin: 0 auto;
  padding: 20px;
}

.header-section {
  display: flex;
  align-items: center;
  gap: 15px;
  margin-bottom: 20px;
}

.back-button {
  flex-shrink: 0;
}

.header-section h2 {
  margin: 0;
  font-size: 24px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.content-section {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.setting-value {
  width: 100%;
}

.info-card,
.settings-card,
.actions-card {
  width: 100%;
}
</style>

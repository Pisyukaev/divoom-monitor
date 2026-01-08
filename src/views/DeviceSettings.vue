<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue';
import { useRouter, useRoute } from 'vue-router';
import { ArrowLeft, Refresh } from '@element-plus/icons-vue';

import { invokeCommand } from '../api/times-gate';
import { commands } from '../constants';
import { useDevice } from '../composables/useDevice';
import type { DivoomDevice, DeviceSettings } from '../types/device';

const router = useRouter();
const route = useRoute();
const { settings, isLoadingSettings, settingsError, fetchDeviceSettings } =
  useDevice();

const deviceId = computed(() => route.params.id as string);
const deviceInfo = ref<DivoomDevice | null>(null);
const isLoadingDevice = ref(false);

// Локальная копия настроек для редактирования
const localSettings = ref<DeviceSettings | null>(null);

// Создаем глубокую копию settings в localSettings
function syncLocalSettings() {
  if (settings.value) {
    localSettings.value = { ...settings.value };
  } else {
    localSettings.value = null;
  }
}

// Синхронизируем localSettings при изменении settings
watch(
  settings,
  () => {
    syncLocalSettings();
  },
  { immediate: true, deep: true }
);

const isLightMode = computed({
  get: () => localSettings.value?.light_switch === 1 || false,
  set: (value: boolean) => {
    if (localSettings.value) {
      localSettings.value.light_switch = value ? 1 : 0;
    }
  },
});

const isMirror = computed({
  get: () => localSettings.value?.mirror_flag === 1 || false,
  set: (value: boolean) => {
    if (localSettings.value) {
      localSettings.value.mirror_flag = value ? 1 : 0;
    }
  },
});

const isRotation = computed({
  get: () => localSettings.value?.rotation_flag === 1 || false,
  set: (value: boolean) => {
    if (localSettings.value) {
      localSettings.value.rotation_flag = value ? 1 : 0;
    }
  },
});

const isCelsius = computed({
  get: () => localSettings.value?.temperature_mode === 0 || false,
  set: (value: boolean) => {
    if (localSettings.value) {
      localSettings.value.temperature_mode = value ? 0 : 1;
    }
  },
});

const is24hours = computed({
  get: () => localSettings.value?.time24_flag === 1 || false,
  set: (value: boolean) => {
    if (localSettings.value) {
      localSettings.value.time24_flag = value ? 1 : 0;
    }
  },
});

const handleChangeOption =
  <K extends keyof DeviceSettings>(
    option: K,
    method: (typeof commands)[number]
  ) =>
  async (value: DeviceSettings[K]) => {
    if (localSettings.value && value !== undefined) {
      localSettings.value[option] = value;

      await invokeCommand(method, {
        ipAddress: deviceId.value,
        value,
      });
    }
  };

function goBack() {
  router.push('/');
}

function handleUpdateSettings() {
  fetchDeviceSettings(deviceId.value);
}

onMounted(() => {
  handleUpdateSettings();
});

// Для отладки можно отслеживать изменения
watch(
  localSettings,
  () => {
    console.log('Local settings changed:', localSettings.value);
  },
  { deep: true }
);
</script>

<template>
  <div class="device-settings-container">
    <div class="header-section">
      <el-button :icon="ArrowLeft" @click="goBack" circle class="back-button" />
      <h2 v-if="deviceInfo">{{ deviceInfo.name }}</h2>
      <h2 v-else>Настройки устройства</h2>
    </div>

    <div
      v-if="!isLoadingDevice"
      v-loading="isLoadingDevice"
      class="content-section"
    >
      <!-- Текущие настройки -->
      <el-card v-loading="isLoadingDevice" class="settings-card" shadow="hover">
        <template #header>
          <div class="card-header">
            <span>Текущие настройки</span>
            <el-button
              :icon="Refresh"
              :loading="isLoadingSettings"
              @click="handleUpdateSettings"
              size="small"
              circle
              :title="'Обновить настройки'"
            />
          </div>
        </template>

        <el-alert
          v-if="settingsError"
          :title="settingsError"
          type="error"
          :closable="false"
          show-icon
          style="margin-bottom: 20px"
        />

        <div v-if="localSettings && !isLoadingSettings">
          <!-- Основные настройки -->
          <el-descriptions title="Основные настройки" :column="1" border>
            <el-descriptions-item
              v-if="localSettings.light_switch !== undefined"
              label="Включить\выключить"
            >
              <el-switch
                @change="(value: boolean) => handleChangeOption('light_switch', 'set_switch_screen')(Number(value))"
                v-model="isLightMode"
              />
            </el-descriptions-item>
            <el-descriptions-item
              v-if="localSettings.brightness !== undefined"
              label="Яркость"
            >
              <div class="setting-value">
                <div
                  style="
                    display: flex;
                    align-items: center;
                    justify-content: space-between;
                  "
                >
                  <el-slider
                    @change="(value: number) => handleChangeOption('brightness', 'set_brightness')(value)"
                    style="width: 80%; padding-right: 10px"
                    :percentage="localSettings.brightness"
                    v-model="localSettings.brightness"
                    :range-end-label="`${localSettings.brightness}%`"
                    :format-tooltip="(value: number) => `${value}%`"
                    :max="100"
                    :min="0"
                    :step="10"
                  />
                  <span>{{ `${localSettings.brightness}%` }}</span>
                </div>
              </div>
            </el-descriptions-item>

            <el-descriptions-item
              v-if="localSettings.mirror_flag !== undefined"
              label="Mirror flag"
            >
              <el-switch
                @change="(value: boolean) => handleChangeOption('mirror_flag', 'set_mirror_mode')(Number(value))"
                v-model="isMirror"
              />
            </el-descriptions-item>
            <el-descriptions-item
              v-if="localSettings.rotation_flag !== null"
              label="Rotation flag"
            >
              <el-switch v-model="isRotation" />
            </el-descriptions-item>
            <el-descriptions-item
              v-if="localSettings.temperature_mode !== undefined"
              label="Temperature mode"
            >
              <el-button-group>
                <el-button
                  :type="isCelsius ? 'primary' : ''"
                  @click="
                    () =>
                      handleChangeOption(
                        'temperature_mode',
                        'set_temperature_mode'
                      )(0)
                  "
                  >Celsius</el-button
                >
                <el-button
                  :type="!isCelsius ? 'primary' : ''"
                  @click="
                    () =>
                      handleChangeOption(
                        'temperature_mode',
                        'set_temperature_mode'
                      )(1)
                  "
                  >Fahrenheit</el-button
                >
              </el-button-group>
            </el-descriptions-item>
            <el-descriptions-item
              v-if="localSettings.time24_flag !== undefined"
              label="24 hours flag"
            >
              <el-switch
                @change="(value: boolean) => handleChangeOption('time24_flag', 'set_24_hours_mode')(Number(value))"
                v-model="is24hours"
              />
            </el-descriptions-item>
          </el-descriptions>
        </div>

        <el-empty
          v-if="!localSettings && !isLoadingSettings && !settingsError"
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

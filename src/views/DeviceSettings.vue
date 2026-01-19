<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { useRouter, useRoute } from 'vue-router';
import { ArrowLeft, Refresh, SwitchButton } from '@element-plus/icons-vue';

import { invokeCommand } from '../api/times-gate';
import { commands } from '../constants';
import { useDevice } from '../composables/useDevice';
import { scanDevices } from '../api/common';
import ScreenManager from '../components/times-gate/ScreenManager.vue';
import type { DivoomDevice, DeviceSettings } from '../types/device';

const router = useRouter();
const route = useRoute();
const { settings, isLoadingSettings, settingsError, fetchDeviceSettings } =
  useDevice();

const deviceId = computed(() => route.params.id as string);
const deviceInfo = ref<DivoomDevice | null>(null);
const isLoadingDevice = ref(false);

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

function createBooleanSetting<K extends keyof DeviceSettings>(
  key: K,
  trueValue: number = 1,
  falseValue: number = 0
) {
  return computed({
    get: () => (settings.value?.[key] as number) === trueValue,
    set: (value: boolean) => {
      if (settings.value) {
        (settings.value[key] as number) = value ? trueValue : falseValue;
      }
    },
  });
}

const isLightMode = createBooleanSetting('light_switch');
const isMirror = createBooleanSetting('mirror_flag');
const is24hours = createBooleanSetting('time24_flag');
const isCelsius = createBooleanSetting('temperature_mode', 0, 1);

const handleChangeOption =
  <K extends keyof DeviceSettings>(
    option: K,
    method: (typeof commands)[number]
  ) =>
    async (value: DeviceSettings[K]) => {
      if (settings.value && value !== undefined) {
        settings.value[option] = value;

        const ip = deviceIp.value || decodeURIComponent(deviceId.value);
        await invokeCommand(method, {
          ipAddress: ip,
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

function handleRebootDevice() {
  invokeCommand('reboot_device', {
    ipAddress: deviceIp.value,
  });
}

onMounted(() => {
  handleUpdateSettings();
  loadDeviceInfo();
});
</script>

<template>
  <div class="device-settings-container">
    <div class="header-section">
      <el-button :icon="ArrowLeft" @click="goBack" circle class="back-button" />
      <h2 v-if="deviceInfo">{{ deviceInfo.name }}</h2>
      <h2 v-else>Настройки устройства</h2>
    </div>

    <div class="content-section">
      <!-- Текущие настройки -->
      <el-card v-loading="isLoadingSettings" class="settings-card" shadow="hover">
        <template #header>
          <div class="card-header">
            <span>Текущие настройки</span>
            <div class="card-header-icons">
              <el-button :icon="Refresh" :loading="isLoadingSettings" @click="handleUpdateSettings" size="small" circle
                :title="'Обновить настройки'" />
              <el-button :icon="SwitchButton" :loading="isLoadingSettings" @click="handleRebootDevice" size="small"
                circle :title="'Перезагрузить устройство'" />
            </div>
          </div>
        </template>

        <el-alert v-if="settingsError" :title="settingsError" type="error" :closable="false" show-icon
          style="margin-bottom: 20px" />

        <div v-if="settings && !isLoadingSettings">
          <!-- Основные настройки -->
          <el-descriptions title="Основные настройки" :column="1" border>
            <el-descriptions-item v-if="settings.light_switch !== undefined" label="Включить\выключить">
              <el-switch
                @change="(value: string | number | boolean) => handleChangeOption('light_switch', 'set_switch_screen')(Number(Boolean(value)))"
                v-model="isLightMode" />
            </el-descriptions-item>
            <el-descriptions-item v-if="settings.brightness !== undefined" label="Яркость">
              <div class="setting-value">
                <div style="
                    display: flex;
                    align-items: center;
                    justify-content: space-between;
                  ">
                  <el-slider
                    @change="(value: number | number[]) => handleChangeOption('brightness', 'set_brightness')(Array.isArray(value) ? value[0] : value)"
                    style="width: 80%; padding-right: 10px" :percentage="settings.brightness"
                    v-model="settings.brightness" :range-end-label="`${settings.brightness}%`"
                    :format-tooltip="(value: number) => `${value}%`" :max="100" :min="0" :step="10" />
                  <span>{{ `${settings.brightness}%` }}</span>
                </div>
              </div>
            </el-descriptions-item>

            <el-descriptions-item v-if="settings.mirror_flag !== undefined" label="Отзеркалить">
              <el-switch
                @change="(value: string | number | boolean) => handleChangeOption('mirror_flag', 'set_mirror_mode')(Number(Boolean(value)))"
                v-model="isMirror" />
            </el-descriptions-item>
            <el-descriptions-item v-if="settings.temperature_mode !== undefined" label="Формат температуры">
              <el-button-group>
                <el-button :type="isCelsius ? 'primary' : ''" @click="
                  () =>
                    handleChangeOption(
                      'temperature_mode',
                      'set_temperature_mode'
                    )(0)
                ">Цельсий</el-button>
                <el-button :type="!isCelsius ? 'primary' : ''" @click="
                  () =>
                    handleChangeOption(
                      'temperature_mode',
                      'set_temperature_mode'
                    )(1)
                ">Фаренгейт</el-button>
              </el-button-group>
            </el-descriptions-item>
            <el-descriptions-item v-if="settings.time24_flag !== undefined" label="24-часовой формат">
              <el-switch
                @change="(value: string | number | boolean) => handleChangeOption('time24_flag', 'set_24_hours_mode')(Number(Boolean(value)))"
                v-model="is24hours" />
            </el-descriptions-item>
          </el-descriptions>
        </div>

        <el-empty v-if="!settings && !isLoadingSettings && !settingsError"
          description="Настройки не загружены. Нажмите 'Обновить' для загрузки." />
      </el-card>

      <!-- Настройка экранов Times Gate -->
      <ScreenManager v-if="isTimesGate && deviceIp" :device-id="deviceId" :device-ip="deviceIp" />

      <!-- Секции для будущего расширения -->
      <el-card class="actions-card" shadow="hover" style="margin-top: 20px">
        <template #header>
          <div class="card-header">
            <span>Управление устройством</span>
          </div>
        </template>
        <el-alert title="Функции управления будут добавлены в будущих версиях" type="info" :closable="false"
          show-icon />
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

.card-header-icons {
  display: flex;
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

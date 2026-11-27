<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useRoute } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { Refresh, SwitchButton, Sunny, CopyDocument, Clock, Histogram, Key } from '@element-plus/icons-vue';

import SettingRow from '../components/ui/SettingRow.vue';
import { useDevice } from '../composables/useDevice';
import { invokeCommand } from '../api/times-gate';
import { commands } from '../constants';
import { parseDeviceIp } from '../utils/device';
import { getLocalToken, isValidLocalToken, saveLocalToken } from '../utils/localToken';
import type { DeviceSettings } from '../types/device';

const { t } = useI18n();
const route = useRoute();

const deviceId = computed(() => route.params.id as string);
const deviceIp = computed(() => parseDeviceIp(deviceId.value) ?? '');

const { settings, isLoadingSettings, settingsError, tokenRequired, fetchDeviceSettings } = useDevice();

function reload() {
  fetchDeviceSettings(deviceId.value);
}

onMounted(reload);

const tokenInput = ref('');
const savedToken = ref('');
const isSavingToken = ref(false);

watch(
  deviceIp,
  (ip) => {
    savedToken.value = ip ? getLocalToken(ip) : '';
    tokenInput.value = savedToken.value;
  },
  { immediate: true },
);

const tokenInvalid = computed(() => tokenInput.value.trim() !== '' && !isValidLocalToken(tokenInput.value.trim()));
const tokenChanged = computed(() => tokenInput.value.trim() !== savedToken.value);

async function storeToken(value: string) {
  if (!deviceIp.value) return;
  isSavingToken.value = true;
  try {
    await saveLocalToken(deviceIp.value, value);
    savedToken.value = value.trim();
    tokenInput.value = savedToken.value;
    ElMessage.success(savedToken.value ? t('control.tokenSaved') : t('control.tokenCleared'));
    reload();
  } catch (e) {
    ElMessage.error(t('control.tokenInvalid'));
    console.error('Failed to save LocalToken:', e);
  } finally {
    isSavingToken.value = false;
  }
}

type NumericKey = 'light_switch' | 'mirror_flag' | 'time24_flag' | 'temperature_mode' | 'brightness';

async function apply(key: NumericKey, method: (typeof commands)[number], value: number) {
  if (!settings.value) return;
  (settings.value as DeviceSettings)[key] = value;
  await invokeCommand(method, { ipAddress: deviceIp.value, value });
}

function toggle(key: NumericKey, method: (typeof commands)[number], on: boolean, onValue = 1, offValue = 0) {
  return apply(key, method, on ? onValue : offValue);
}

const isOn = computed(() => settings.value?.light_switch === 1);
// Unknown power state (no token) must not lock the slider: only a known "off" does.
const isKnownOff = computed(() => settings.value?.light_switch === 0);
// Two-way binding is required: el-slider's `change` event reports its
// modelValue, so a one-way :model-value makes it send the old brightness.
const brightness = computed({
  get: () => settings.value?.brightness ?? 0,
  set: (v: number) => {
    if (settings.value) settings.value.brightness = v;
  },
});
const isMirror = computed(() => settings.value?.mirror_flag === 1);
// undefined = unknown (no token): no option is highlighted.
const timeFormat = computed(() => settings.value?.time24_flag ?? undefined);
const temperatureMode = computed(() => settings.value?.temperature_mode ?? undefined);
</script>

<template>
  <div class="control">
    <el-alert v-if="settingsError" :title="settingsError" type="error" :closable="false" show-icon>
      <el-button size="small" @click="reload">{{ t('common.retry') }}</el-button>
    </el-alert>
    <el-alert v-else-if="tokenRequired" :title="savedToken ? t('control.tokenWrong') : t('control.tokenRequired')"
      :description="t('control.valuesUnknown')" type="warning" :closable="false" show-icon />

    <div v-loading="isLoadingSettings && !settings" class="grid">
      <section class="app-card">
        <div class="app-card-header">
          <span class="app-card-title">{{ t('control.screen') }}</span>
          <el-tooltip :content="t('control.refresh')" placement="top">
            <el-button :icon="Refresh" text circle :loading="isLoadingSettings" @click="reload" />
          </el-tooltip>
        </div>
        <div class="app-card-body">
          <template v-if="settings">
            <SettingRow :icon="SwitchButton" :title="t('control.power')" :description="t('control.powerDesc')">
              <el-switch :model-value="isOn" @change="(v) => toggle('light_switch', 'set_switch_screen', Boolean(v))" />
            </SettingRow>
            <SettingRow :icon="Sunny" :title="t('control.brightness')" stacked>
              <div class="brightness">
                <el-slider v-model="brightness" :min="0" :max="100" :step="5"
                  :disabled="isKnownOff" :format-tooltip="(v: number) => `${v}%`"
                  @change="(v) => apply('brightness', 'set_brightness', Array.isArray(v) ? v[0] : v)" />
                <span class="brightness-value">{{ settings.brightness === null ? '—' : `${settings.brightness}%` }}</span>
              </div>
            </SettingRow>
          </template>
          <div v-else-if="!isLoadingSettings" class="placeholder app-muted">{{ t('control.notLoaded') }}</div>
        </div>
      </section>

      <section class="app-card">
        <div class="app-card-header">
          <span class="app-card-title">{{ t('control.display') }}</span>
        </div>
        <div class="app-card-body">
          <template v-if="settings">
            <SettingRow :icon="CopyDocument" :title="t('control.mirror')" :description="t('control.mirrorDesc')">
              <el-switch :model-value="isMirror"
                @change="(v) => toggle('mirror_flag', 'set_mirror_mode', Boolean(v))" />
            </SettingRow>
            <SettingRow :icon="Clock" :title="t('control.timeFormat')">
              <el-segmented :model-value="timeFormat" :options="[
                { label: '12h', value: 0 },
                { label: '24h', value: 1 },
              ]" @change="(v: number) => apply('time24_flag', 'set_24_hours_mode', v)" />
            </SettingRow>
            <SettingRow :icon="Histogram" :title="t('control.temperature')">
              <el-segmented :model-value="temperatureMode" :options="[
                { label: '°C', value: 0 },
                { label: '°F', value: 1 },
              ]" @change="(v: number) => apply('temperature_mode', 'set_temperature_mode', v)" />
            </SettingRow>
          </template>
          <div v-else-if="!isLoadingSettings" class="placeholder app-muted">{{ t('control.notLoaded') }}</div>
        </div>
      </section>

      <section class="app-card">
        <div class="app-card-header">
          <span class="app-card-title">{{ t('control.access') }}</span>
        </div>
        <div class="app-card-body">
          <SettingRow :icon="Key" :title="t('control.token')" :description="t('control.tokenDesc')" stacked>
            <div class="token">
              <el-input v-model="tokenInput" type="password" show-password inputmode="numeric" maxlength="10"
                autocomplete="off" :placeholder="t('control.tokenPlaceholder')" :class="{ 'is-error': tokenInvalid }"
                @keyup.enter="!tokenInvalid && tokenChanged && storeToken(tokenInput)" />
              <el-button type="primary" :disabled="tokenInvalid || !tokenChanged" :loading="isSavingToken"
                @click="storeToken(tokenInput)">
                {{ t('control.tokenSave') }}
              </el-button>
              <el-button v-if="savedToken" :disabled="isSavingToken" @click="storeToken('')">
                {{ t('control.tokenClear') }}
              </el-button>
            </div>
            <div v-if="tokenInvalid" class="token-error">{{ t('control.tokenInvalid') }}</div>
          </SettingRow>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.control {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(340px, 1fr));
  gap: 16px;
  align-items: start;
  min-height: 120px;
}

.brightness {
  display: flex;
  align-items: center;
  gap: 16px;
}

.brightness .el-slider {
  flex: 1;
}

.brightness-value {
  min-width: 44px;
  text-align: right;
  font-variant-numeric: tabular-nums;
  font-weight: 600;
}

.token {
  display: flex;
  gap: 8px;
}

.token .el-input {
  flex: 1;
}

.token .is-error :deep(.el-input__wrapper) {
  box-shadow: 0 0 0 1px var(--el-color-danger) inset;
}

.token-error {
  margin-top: 6px;
  font-size: 12px;
  color: var(--el-color-danger);
}

.placeholder {
  padding: 24px 0;
  text-align: center;
}
</style>

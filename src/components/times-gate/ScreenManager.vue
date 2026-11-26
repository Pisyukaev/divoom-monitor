<script setup lang="ts">
import { ref, onBeforeMount, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { Promotion } from '@element-plus/icons-vue';
import ScreenEditor from './ScreenEditor.vue';
import ScreenPreview from './ScreenPreview.vue';
import { TEXT_IDS } from '../../constants';
import { sendConfigsToDevice } from '../../composables/useAutoSendConfig';
import type { ScreenConfig, ScreenConfigs } from '../../types/screen';

const SCREEN_COUNT = 5;

const { t } = useI18n();

const props = defineProps<{
  deviceId: string;
  deviceIp: string;
}>();

const activeScreen = ref(0);
const screenConfigs = ref<ScreenConfigs>({});
const isSending = ref(false);

function emptyConfig(screenIndex: number): ScreenConfig {
  return { screenIndex, texts: [], textIds: TEXT_IDS };
}

function loadConfigs() {
  const configs: ScreenConfigs = {};
  const stored = localStorage.getItem(`screen_configs_${props.deviceId}`);
  if (stored) {
    try {
      Object.assign(configs, JSON.parse(stored));
    } catch (error) {
      console.error('Error loading configs:', error);
    }
  }
  for (let i = 0; i < SCREEN_COUNT; i++) {
    configs[i] ??= emptyConfig(i);
  }
  screenConfigs.value = configs;
}

function saveConfigs() {
  try {
    localStorage.setItem(`screen_configs_${props.deviceId}`, JSON.stringify(screenConfigs.value));
  } catch (error) {
    console.error('Error saving configs:', error);
    ElMessage.error(t('screenManager.configSaveError'));
  }
}

function handleConfigUpdate(screenIndex: number, config: ScreenConfig) {
  screenConfigs.value[screenIndex] = config;
  saveConfigs();
}

async function handleSendAllToDevice() {
  isSending.value = true;
  try {
    await sendConfigsToDevice(props.deviceIp, screenConfigs.value);
    ElMessage.success(t('screenManager.allSent'));
  } catch (error) {
    console.error('Error sending configs to device:', error);
    ElMessage.error(t('screenManager.sendError'));
  } finally {
    isSending.value = false;
  }
}

onBeforeMount(loadConfigs);
watch(() => props.deviceId, loadConfigs);
</script>

<template>
  <div class="screen-manager">
    <section class="app-card">
      <div class="app-card-header">
        <div>
          <div class="app-card-title">{{ t('screenManager.title') }}</div>
          <div class="app-muted hint">{{ t('screenManager.hint') }}</div>
        </div>
        <el-button type="primary" :icon="Promotion" :loading="isSending" @click="handleSendAllToDevice">
          {{ t('screenManager.sendToDevice') }}
        </el-button>
      </div>

      <div class="app-card-body">
        <div class="device-strip" role="tablist">
          <button v-for="i in SCREEN_COUNT" :key="i" class="strip-screen" :class="{ active: activeScreen === i - 1 }"
            role="tab" :aria-selected="activeScreen === i - 1" @click="activeScreen = i - 1">
            <span class="strip-preview">
              <ScreenPreview :config="screenConfigs[i - 1] || emptyConfig(i - 1)" :scale="88" compact />
            </span>
            <span class="strip-label">{{ t('screenManager.screenN', { n: i }) }}</span>
          </button>
        </div>
      </div>
    </section>

    <ScreenEditor :key="`${deviceId}-${activeScreen}`" :config="screenConfigs[activeScreen] || emptyConfig(activeScreen)"
      :device-ip="deviceIp" @update:config="(config) => handleConfigUpdate(activeScreen, config)" />
  </div>
</template>

<style scoped>
.screen-manager {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.hint {
  font-size: 13px;
  margin-top: 2px;
}

.device-strip {
  display: flex;
  justify-content: safe center;
  gap: 14px;
  padding: 18px;
  border-radius: var(--app-radius);
  background:
    radial-gradient(circle at 20% 0%, rgba(124, 124, 240, 0.18), transparent 55%),
    var(--app-screen-bezel);
  overflow-x: auto;
}

.strip-screen {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 6px;
  border: 2px solid transparent;
  border-radius: 14px;
  background: transparent;
  cursor: pointer;
  color: rgba(255, 255, 255, 0.6);
  font: inherit;
  transition: border-color 0.15s, background-color 0.15s;
}

.strip-screen:hover {
  background: rgba(255, 255, 255, 0.04);
}

.strip-screen.active {
  border-color: var(--el-color-primary);
  background: rgba(124, 124, 240, 0.12);
  color: #fff;
}

.strip-preview {
  pointer-events: none;
  border-radius: 10px;
  overflow: hidden;
  display: block;
}

.strip-label {
  font-size: 12px;
  font-weight: 500;
}
</style>

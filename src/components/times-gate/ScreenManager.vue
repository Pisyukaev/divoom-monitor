<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import { ElMessage } from 'element-plus';
import { invoke } from '@tauri-apps/api/core';
import ScreenEditor from './ScreenEditor.vue';
import type { ScreenConfig, ScreenConfigs } from '../../types/screen';

const props = defineProps<{
  deviceId: string;
  deviceIp: string;
}>();

const activeTab = ref('0');
const screenConfigs = ref<ScreenConfigs>({});

// Initialize default configs for 5 screens
function initializeConfigs() {
  const configs: ScreenConfigs = {};
  for (let i = 0; i < 5; i++) {
    configs[i] = {
      screenIndex: i,
      texts: [],
    };
  }
  return configs;
}

function loadConfigs() {
  const stored = localStorage.getItem(`screen_configs_${props.deviceId}`);
  if (stored) {
    try {
      screenConfigs.value = JSON.parse(stored);
      // Ensure all 5 screens exist
      for (let i = 0; i < 5; i++) {
        if (!screenConfigs.value[i]) {
          screenConfigs.value[i] = {
            screenIndex: i,
            texts: [],
          };
        }
      }
    } catch (error) {
      console.error('Error loading configs:', error);
      screenConfigs.value = initializeConfigs();
    }
  } else {
    screenConfigs.value = initializeConfigs();
  }
}

function saveConfigs() {
  try {
    localStorage.setItem(
      `screen_configs_${props.deviceId}`,
      JSON.stringify(screenConfigs.value)
    );
    ElMessage.success('Конфигурация сохранена');
  } catch (error) {
    console.error('Error saving configs:', error);
    ElMessage.error('Ошибка сохранения конфигурации');
  }
}

function handleConfigUpdate(screenIndex: number, config: ScreenConfig) {
  screenConfigs.value[screenIndex] = config;
  // Auto-save on change
  saveConfigs();
}

async function handleSendAllToDevice() {
  try {
    ElMessage.info('Отправка всех конфигураций на устройство...');

    // Send images and texts for each screen
    for (let i = 0; i < 5; i++) {
      const config = screenConfigs.value[i];
      if (!config) continue;

      // Send image if present
      if (config.image) {
        try {
          if (config.image.type === 'url') {
            await invoke('upload_image_from_url', {
              ipAddress: props.deviceIp,
              screenIndex: i,
              url: config.image.source,
            });
          } else if (config.image.type === 'local') {
            await invoke('upload_image_from_file', {
              ipAddress: props.deviceIp,
              screenIndex: i,
              filePath: config.image.source,
            });
          }
        } catch (error) {
          console.error(`Error sending image for screen ${i}:`, error);
          ElMessage.warning(`Ошибка отправки изображения для экрана ${i + 1}`);
        }
      }

      // Send all texts for this screen
      for (const text of config.texts) {
        try {
          await invoke('set_screen_text', {
            ipAddress: props.deviceIp,
            screenIndex: i,
            textConfig: {
              id: text.id,
              content: text.content,
              x: text.x,
              y: text.y,
              font: text.font,
              color: text.color?.toUpperCase(),
              alignment: text.alignment,
              text_width: text.textWidth,
            },
          });
        } catch (error) {
          console.error(`Error sending text for screen ${i}:`, error);
          ElMessage.warning(`Ошибка отправки текста для экрана ${i + 1}`);
        }
      }
    }

    ElMessage.success('Все конфигурации отправлены на устройство');
  } catch (error) {
    console.error('Error sending configs to device:', error);
    ElMessage.error('Ошибка отправки конфигураций на устройство');
  }
}

onMounted(() => {
  loadConfigs();
});

watch(
  () => props.deviceId,
  () => {
    loadConfigs();
  }
);
</script>

<template>
  <div class="screen-manager">
    <el-card shadow="hover">
      <template #header>
        <div class="manager-header">
          <span>Настройка экранов Times Gate</span>
          <div class="header-actions">
            <el-button type="primary" @click="saveConfigs">
              Сохранить
            </el-button>
            <el-button type="success" @click="handleSendAllToDevice">
              Отправить на устройство
            </el-button>
          </div>
        </div>
      </template>

      <el-tabs v-model="activeTab" type="border-card">
        <el-tab-pane v-for="i in 5" :key="i - 1" :label="`Экран ${i}`" :name="String(i - 1)">
          <ScreenEditor :config="screenConfigs[i - 1] || { screenIndex: i - 1, texts: [] }" :device-ip="deviceIp"
            @update:config="(config) => handleConfigUpdate(i - 1, config)" />
        </el-tab-pane>
      </el-tabs>
    </el-card>
  </div>
</template>

<style scoped>
.screen-manager {
  width: 100%;
}

.manager-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.header-actions {
  display: flex;
  gap: 10px;
}

:deep(.el-tabs__content) {
  padding: 20px;
}
</style>

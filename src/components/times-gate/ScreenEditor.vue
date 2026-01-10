<script setup lang="ts">
import { ref, computed } from 'vue';
import { ElMessage } from 'element-plus';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import ScreenPreview from './ScreenPreview.vue';
import type { ScreenConfig, TextElement, ScreenImage } from '../../types/screen';

const props = defineProps<{
  config: ScreenConfig;
  deviceIp: string;
}>();

const emit = defineEmits<{
  'update:config': [config: ScreenConfig];
}>();

const imageUrlInput = ref('');
const isLoadingImage = ref(false);
const newTextContent = ref('');
const newTextColor = ref('#ffffff');
const newTextSize = ref(16);
const newTextAlignment = ref<'left' | 'center' | 'right'>('left');

const localConfig = computed({
  get: () => props.config,
  set: (value) => emit('update:config', value),
});

function generateTextId(): string {
  return `text_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
}

async function handleLoadLocalImage() {
  try {
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: 'Images',
          extensions: ['png', 'jpg', 'jpeg', 'gif', 'bmp', 'webp'],
        },
      ],
    });

    if (!selected || Array.isArray(selected)) {
      return;
    }

    isLoadingImage.value = true;
    try {
      await invoke('upload_image_from_file', {
        ipAddress: props.deviceIp,
        screenIndex: props.config.screenIndex,
        filePath: selected,
      });

      localConfig.value = {
        ...localConfig.value,
        image: {
          type: 'local',
          source: selected,
        },
      };

      ElMessage.success('Изображение загружено');
    } catch (error) {
      console.error('Error uploading image:', error);
      ElMessage.error(`Ошибка загрузки изображения: ${error}`);
    } finally {
      isLoadingImage.value = false;
    }
  } catch (error) {
    console.error('Error selecting file:', error);
  }
}

async function handleLoadImageFromUrl() {
  if (!imageUrlInput.value.trim()) {
    ElMessage.warning('Введите URL изображения');
    return;
  }

  try {
    isLoadingImage.value = true;
    await invoke('upload_image_from_url', {
      ipAddress: props.deviceIp,
      screenIndex: props.config.screenIndex,
      url: imageUrlInput.value,
    });

    localConfig.value = {
      ...localConfig.value,
      image: {
        type: 'url',
        source: imageUrlInput.value,
      },
    };

    ElMessage.success('Изображение загружено');
    imageUrlInput.value = '';
  } catch (error) {
    console.error('Error uploading image:', error);
    ElMessage.error(`Ошибка загрузки изображения: ${error}`);
  } finally {
    isLoadingImage.value = false;
  }
}

function handleRemoveImage() {
  localConfig.value = {
    ...localConfig.value,
    image: undefined,
  };
}

function handleAddText() {
  if (!newTextContent.value.trim()) {
    ElMessage.warning('Введите текст');
    return;
  }

  const newText: TextElement = {
    id: generateTextId(),
    content: newTextContent.value,
    x: 10,
    y: 20,
    fontSize: newTextSize.value,
    color: newTextColor.value,
    alignment: newTextAlignment.value,
  };

  localConfig.value = {
    ...localConfig.value,
    texts: [...localConfig.value.texts, newText],
  };

  newTextContent.value = '';
  ElMessage.success('Текст добавлен');
}

function handleRemoveText(textId: string) {
  localConfig.value = {
    ...localConfig.value,
    texts: localConfig.value.texts.filter((t) => t.id !== textId),
  };
}

function handleUpdateTextPosition(textId: string, x: number, y: number) {
  localConfig.value = {
    ...localConfig.value,
    texts: localConfig.value.texts.map((t) =>
      t.id === textId ? { ...t, x: Math.round(x), y: Math.round(y) } : t
    ),
  };
}

function hexToRgb(hex: string): string {
  // Remove # if present
  hex = hex.replace('#', '');
  
  // Parse hex to RGB
  const r = parseInt(hex.substring(0, 2), 16);
  const g = parseInt(hex.substring(2, 4), 16);
  const b = parseInt(hex.substring(4, 6), 16);
  
  return `${r},${g},${b}`;
}

async function handleSendTextToDevice(text: TextElement) {
  try {
    const color = text.color ? hexToRgb(text.color) : '255,255,255';
    
    await invoke('set_screen_text', {
      ipAddress: props.deviceIp,
      screenIndex: props.config.screenIndex,
      textConfig: {
        content: text.content,
        x: text.x,
        y: text.y,
        font_size: text.fontSize,
        color: color,
        alignment: text.alignment,
      },
    });
    ElMessage.success('Текст отправлен на устройство');
  } catch (error) {
    console.error('Error sending text:', error);
    ElMessage.error(`Ошибка отправки текста: ${error}`);
  }
}
</script>

<template>
  <div class="screen-editor">
    <div class="editor-controls">
      <el-card shadow="hover">
        <template #header>
          <span>Изображение</span>
        </template>

        <div class="control-section">
          <el-button
            type="primary"
            @click="handleLoadLocalImage"
            :loading="isLoadingImage"
            style="width: 100%; margin-bottom: 10px"
          >
            Загрузить с компьютера
          </el-button>

          <el-input
            v-model="imageUrlInput"
            placeholder="URL изображения"
            style="margin-bottom: 10px"
          >
            <template #append>
              <el-button
                @click="handleLoadImageFromUrl"
                :loading="isLoadingImage"
                :disabled="!imageUrlInput.trim()"
              >
                Загрузить
              </el-button>
            </template>
          </el-input>

          <el-button
            v-if="localConfig.image"
            type="danger"
            @click="handleRemoveImage"
            style="width: 100%"
          >
            Удалить изображение
          </el-button>
        </div>
      </el-card>

      <el-card shadow="hover" style="margin-top: 20px">
        <template #header>
          <span>Текст</span>
        </template>

        <div class="control-section">
          <el-input
            v-model="newTextContent"
            placeholder="Введите текст"
            style="margin-bottom: 10px"
          />

          <div style="display: flex; gap: 10px; margin-bottom: 10px">
            <el-input-number
              v-model="newTextSize"
              :min="8"
              :max="32"
              label="Размер"
              style="flex: 1"
            />
            <el-color-picker v-model="newTextColor" />
          </div>

          <el-radio-group v-model="newTextAlignment" style="margin-bottom: 10px">
            <el-radio-button label="left">Слева</el-radio-button>
            <el-radio-button label="center">По центру</el-radio-button>
            <el-radio-button label="right">Справа</el-radio-button>
          </el-radio-group>

          <el-button
            type="primary"
            @click="handleAddText"
            :disabled="!newTextContent.trim()"
            style="width: 100%"
          >
            Добавить текст
          </el-button>
        </div>
      </el-card>

      <el-card shadow="hover" style="margin-top: 20px">
        <template #header>
          <span>Элементы текста</span>
        </template>

        <div class="text-list">
          <div
            v-for="text in localConfig.texts"
            :key="text.id"
            class="text-item"
          >
            <div class="text-item-content">
              <span class="text-preview">{{ text.content }}</span>
              <span class="text-position"
                >({{ Math.round(text.x) }}, {{ Math.round(text.y) }})</span
              >
            </div>
            <div class="text-item-actions">
              <el-button
                size="small"
                @click="handleSendTextToDevice(text)"
                type="success"
              >
                Отправить
              </el-button>
              <el-button
                size="small"
                type="danger"
                @click="handleRemoveText(text.id)"
              >
                Удалить
              </el-button>
            </div>
          </div>
          <el-empty
            v-if="localConfig.texts.length === 0"
            description="Нет текстовых элементов"
            :image-size="60"
          />
        </div>
      </el-card>
    </div>

    <div class="editor-preview">
      <el-card shadow="hover">
        <template #header>
          <span>Предпросмотр экрана {{ config.screenIndex + 1 }}</span>
        </template>
        <div class="preview-container">
          <ScreenPreview
            :config="localConfig"
            :scale="400"
            @update:text-position="handleUpdateTextPosition"
          />
        </div>
      </el-card>
    </div>
  </div>
</template>

<style scoped>
.screen-editor {
  display: flex;
  gap: 20px;
  height: 100%;
}

.editor-controls {
  flex: 0 0 350px;
  overflow-y: auto;
  max-height: calc(100vh - 200px);
}

.editor-preview {
  flex: 1;
  display: flex;
  align-items: flex-start;
}

.preview-container {
  display: flex;
  justify-content: center;
  padding: 20px;
}

.control-section {
  display: flex;
  flex-direction: column;
}

.text-list {
  max-height: 300px;
  overflow-y: auto;
}

.text-item {
  padding: 10px;
  border: 1px solid var(--el-border-color-light);
  border-radius: 4px;
  margin-bottom: 8px;
  background-color: var(--el-bg-color);
}

.text-item-content {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-bottom: 8px;
}

.text-preview {
  font-weight: 500;
  color: var(--el-text-color-primary);
}

.text-position {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.text-item-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}
</style>

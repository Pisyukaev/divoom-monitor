<script setup lang="ts">
import { ref } from 'vue';
import { ElMessage } from 'element-plus';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import ScreenPreview from './ScreenPreview.vue';
import TextElement from './TextElement.vue';
import type { ScreenConfig, TextElement as TextElementType } from '../../types/screen';

const props = defineProps<{
  config: ScreenConfig;
  deviceIp: string;
}>();

const emit = defineEmits<{
  'update:config': [config: ScreenConfig];
}>();

const imageUrlInput = ref('');
const isLoadingImage = ref(false);
const selectedText = ref<TextElementType | null>(null);

const localConfig = ref<ScreenConfig>(props.config);

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
      emit('update:config', localConfig.value);
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
    emit('update:config', localConfig.value);
  }
}

function handleRemoveImage() {
  localConfig.value = {
    ...localConfig.value,
    image: undefined,
  };

  emit('update:config', localConfig.value);
}

function handleAddText(text: TextElementType) {
  localConfig.value = {
    ...localConfig.value,
    texts: [...localConfig.value.texts, text],
  };
  handleSendTextToDevice(text);
  ElMessage.success('Текст добавлен');
  emit('update:config', localConfig.value);
}

function handleRemoveText(textId: number) {
  if (selectedText.value?.id === textId) {
    selectedText.value = null;
  }

  localConfig.value = {
    ...localConfig.value,
    texts: localConfig.value.texts.filter((t) => t.id !== textId),
  };

  emit('update:config', localConfig.value);
}

function handleTextClick(textId: number | null) {
  if (textId === null) {
    selectedText.value = null;
    return;
  }

  const text = localConfig.value.texts.find((t) => t.id === textId);

  selectedText.value = text || null;
}

function handleUpdateSelectedText(text: TextElementType) {
  if (selectedText.value === null) {
    return;
  }

  localConfig.value = {
    ...localConfig.value,
    texts: localConfig.value.texts.map((t) =>
      t.id === text.id
        ? {
          ...t,
          ...text
        }
        : t
    ),
  }
}

function handleUpdateTextPosition(textId: number, x: number, y: number) {
  localConfig.value = {
    ...localConfig.value,
    texts: localConfig.value.texts.map((t) =>
      t.id === textId ? { ...t, x: Math.round(x), y: Math.round(y) } : t
    ),
  };
  if (selectedText.value?.id === textId) {
    selectedText.value.x = Math.round(x);
    selectedText.value.y = Math.round(y);
  }
}

async function handleSendTextToDevice(text: TextElementType) {
  try {
    await invoke('set_screen_text', {
      ipAddress: props.deviceIp,
      screenIndex: props.config.screenIndex,
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
    ElMessage.success('Текст отправлен на устройство');
  } catch (error) {
    console.error('Error sending text:', error);
    ElMessage.error(`Ошибка отправки текста: ${error}`);
  } finally {
    emit('update:config', localConfig.value);
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
          <el-button type="primary" @click="handleLoadLocalImage" :loading="isLoadingImage"
            style="width: 100%; margin-bottom: 10px">
            Загрузить с компьютера
          </el-button>

          <el-input v-model="imageUrlInput" placeholder="URL изображения" style="margin-bottom: 10px">
            <template #append>
              <el-button @click="handleLoadImageFromUrl" :loading="isLoadingImage" :disabled="!imageUrlInput.trim()">
                Загрузить
              </el-button>
            </template>
          </el-input>

          <el-button v-if="localConfig.image" type="danger" @click="handleRemoveImage" style="width: 100%">
            Удалить изображение
          </el-button>
        </div>
      </el-card>

      <TextElement :text="selectedText" @update:text="handleUpdateSelectedText" @submit:text="handleSendTextToDevice"
        @add:text="handleAddText" />
    </div>

    <div class="editor-preview">
      <el-card shadow="hover">
        <template #header>
          <span>Предпросмотр экрана {{ config.screenIndex + 1 }}</span>
        </template>
        <div class="preview-container">
          <ScreenPreview :config="localConfig" :scale="400" :selected-text="selectedText"
            @update:text-position="handleUpdateTextPosition" @text-delete="handleRemoveText"
            @text-click="handleTextClick" />
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

.radio-group {
  display: flex;
  flex-direction: row;
  justify-content: center;
  align-items: center;
  flex-wrap: nowrap;
  margin-bottom: 10px;
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
</style>

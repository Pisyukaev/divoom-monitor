<script setup lang="ts">
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { FolderOpened, Link, Delete, Picture } from '@element-plus/icons-vue';
import ScreenPreview from './ScreenPreview.vue';
// import TextElement from './TextElement.vue';
import type { ScreenConfig, TextElement as TextElementType } from '../../types/screen';

const { t } = useI18n();

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

      ElMessage.success(t('screenEditor.imageLoaded'));
    } catch (error) {
      console.error('Error uploading image:', error);
      ElMessage.error(t('screenEditor.imageLoadError', { error: String(error) }));
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
    ElMessage.warning(t('screenEditor.enterImageUrl'));
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

    ElMessage.success(t('screenEditor.imageLoaded'));
    imageUrlInput.value = '';
  } catch (error) {
    console.error('Error uploading image:', error);
    ElMessage.error(t('screenEditor.imageLoadError', { error: String(error) }));
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

// function handleAddText(text: TextElementType) {
//   localConfig.value = {
//     ...localConfig.value,
//     texts: [...localConfig.value.texts, { ...text, color: text.color?.toUpperCase() }],
//   };
//   handleSendTextToDevice(text);
//   ElMessage.success(t('screenEditor.textAdded'));
//   emit('update:config', localConfig.value);
// }

function handleRemoveText(textId: number) {
  if (selectedText.value?.id === textId) {
    selectedText.value = null;
  }

  localConfig.value = {
    ...localConfig.value,
    texts: localConfig.value.texts.filter((t) => t.id !== textId),
    textIds: [...localConfig.value.textIds, textId],
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

// function handleUpdateSelectedText(text: TextElementType) {
//   if (selectedText.value === null) {
//     return;
//   }

//   localConfig.value = {
//     ...localConfig.value,
//     texts: localConfig.value.texts.map((t) =>
//       t.id === text.id
//         ? {
//           ...t,
//           ...text
//         }
//         : t
//     ),
//   }
// }

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

// async function handleSendTextToDevice(text: TextElementType) {
//   try {
//     await invoke('set_screen_text', {
//       ipAddress: props.deviceIp,
//       screenIndex: props.config.screenIndex,
//       textConfig: {
//         id: text.id,
//         content: text.content,
//         x: text.x,
//         y: text.y,
//         font: text.font,
//         color: text.color?.toUpperCase(),
//         alignment: text.alignment,
//         text_width: text.textWidth,
//       },
//     });
//     ElMessage.success(t('screenEditor.textSent'));
//   } catch (error) {
//     console.error('Error sending text:', error);
//     ElMessage.error(t('screenEditor.textSendError', { error: String(error) }));
//   } finally {
//     emit('update:config', localConfig.value);
//   }
// }
const imageSourceLabel = computed(() => {
  const image = localConfig.value.image;
  if (!image) return null;
  if (image.type === 'url') return image.source;
  return image.source.split(/[\\/]/).pop() ?? image.source;
});
</script>

<template>
  <section class="screen-editor app-card">
    <div class="preview-pane">
      <ScreenPreview :config="localConfig" :scale="320" :selected-text="selectedText"
        @update:text-position="handleUpdateTextPosition" @text-delete="handleRemoveText"
        @text-click="handleTextClick" />
      <span class="app-muted preview-caption">
        {{ t('screenEditor.screenPreview', { n: config.screenIndex + 1 }) }} · 128 × 128
      </span>
    </div>

    <div class="controls-pane">
      <div class="app-card-title">{{ t('screenEditor.image') }}</div>

      <div v-if="localConfig.image" class="current-image">
        <el-icon class="current-icon">
          <Picture />
        </el-icon>
        <div class="current-text">
          <div class="app-muted current-label">{{ t('screenEditor.current') }}</div>
          <div class="current-source" :title="localConfig.image.source">{{ imageSourceLabel }}</div>
        </div>
        <el-tooltip :content="t('screenEditor.removeImage')" placement="top">
          <el-button :icon="Delete" text circle type="danger" @click="handleRemoveImage" />
        </el-tooltip>
      </div>

      <button class="dropzone" :disabled="isLoadingImage" @click="handleLoadLocalImage">
        <el-icon class="dropzone-icon" :class="{ 'is-loading': isLoadingImage }">
          <FolderOpened />
        </el-icon>
        <span class="dropzone-title">{{ t('screenEditor.loadFromComputer') }}</span>
        <span class="app-muted dropzone-hint">{{ t('screenEditor.formats') }}</span>
      </button>

      <div class="divider app-muted"><span>{{ t('screenEditor.orByUrl') }}</span></div>

      <div class="url-row">
        <el-input v-model="imageUrlInput" :prefix-icon="Link" :placeholder="t('screenEditor.imageUrl')" clearable
          @keyup.enter="handleLoadImageFromUrl" />
        <el-button :loading="isLoadingImage" :disabled="!imageUrlInput.trim()" @click="handleLoadImageFromUrl">
          {{ t('screenEditor.load') }}
        </el-button>
      </div>

      <p class="app-muted note">{{ t('screenEditor.instantNote') }}</p>
    </div>
  </section>
</template>

<style scoped>
.screen-editor {
  display: grid;
  grid-template-columns: auto minmax(280px, 1fr);
  gap: 32px;
  padding: 28px;
}

.preview-pane {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 18px;
  padding: 6px;
}

.preview-caption {
  font-size: 12.5px;
}

.controls-pane {
  display: flex;
  flex-direction: column;
  gap: 14px;
  min-width: 0;
}

.current-image {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 8px 10px 14px;
  border-radius: var(--app-radius-sm);
  background: var(--el-fill-color-light);
}

.current-icon {
  font-size: 20px;
  color: var(--el-color-primary);
}

.current-text {
  flex: 1;
  min-width: 0;
}

.current-label {
  font-size: 12px;
}

.current-source {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-weight: 500;
}

.dropzone {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 26px 16px;
  border: 1.5px dashed var(--el-border-color);
  border-radius: var(--app-radius);
  background: var(--app-surface-muted);
  color: var(--el-text-color-primary);
  font: inherit;
  cursor: pointer;
  transition: border-color 0.15s, background-color 0.15s;
}

.dropzone:hover:not(:disabled) {
  border-color: var(--el-color-primary);
  background: var(--el-color-primary-light-9);
}

.dropzone:disabled {
  cursor: progress;
  opacity: 0.7;
}

.dropzone-icon {
  font-size: 26px;
  color: var(--el-color-primary);
  margin-bottom: 4px;
}

.dropzone-title {
  font-weight: 600;
}

.dropzone-hint {
  font-size: 12px;
}

.divider {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 12px;
}

.divider::before,
.divider::after {
  content: '';
  flex: 1;
  height: 1px;
  background: var(--el-border-color-light);
}

.url-row {
  display: flex;
  gap: 8px;
}

.note {
  margin: 0;
  font-size: 12.5px;
}

@media (max-width: 980px) {
  .screen-editor {
    grid-template-columns: 1fr;
  }
}
</style>

<script setup lang="ts">
import { ref, computed, onBeforeUnmount } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { open } from '@tauri-apps/plugin-dialog';
import { FolderOpened, Link, Delete, Picture } from '@element-plus/icons-vue';
import ScreenPreview from './ScreenPreview.vue';
import TextElement from './TextElement.vue';
import { sendScreen } from '../../utils/screenRender';
import type { ScreenConfig, ScreenImage, TextElement as TextElementType } from '../../types/screen';

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

// The whole screen (image + texts) is rendered on the PC and sent as one
// picture (utils/screenRender.ts). Text edits are sent shortly after the last
// change, so typing or dragging produces one request instead of dozens.
const SEND_DELAY_MS = 400;
let sendTimer: ReturnType<typeof setTimeout> | undefined;

function scheduleSend() {
  clearTimeout(sendTimer);
  sendTimer = setTimeout(sendNow, SEND_DELAY_MS);
}

async function sendNow() {
  clearTimeout(sendTimer);
  sendTimer = undefined;
  try {
    await sendScreen(props.deviceIp, localConfig.value);
  } catch (error) {
    console.error('Error sending screen:', error);
    ElMessage.error(t('screenEditor.textSendError', { error: String(error) }));
  }
}

onBeforeUnmount(() => {
  // Don't lose the last edit when leaving the page.
  if (sendTimer !== undefined) sendNow();
});

function commit(config: ScreenConfig) {
  localConfig.value = config;
  emit('update:config', config);
}

/** Sends the screen with the new image first and saves it only if that worked. */
async function applyImage(image: ScreenImage): Promise<boolean> {
  isLoadingImage.value = true;
  try {
    const next = { ...localConfig.value, image };
    await sendScreen(props.deviceIp, next);
    clearTimeout(sendTimer); // this send already includes any pending text edit
    sendTimer = undefined;
    commit(next);
    ElMessage.success(t('screenEditor.imageLoaded'));
    return true;
  } catch (error) {
    console.error('Error uploading image:', error);
    ElMessage.error(t('screenEditor.imageLoadError', { error: String(error) }));
    return false;
  } finally {
    isLoadingImage.value = false;
  }
}

async function handleLoadLocalImage() {
  let selected: string | string[] | null;
  try {
    selected = await open({
      multiple: false,
      filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'gif', 'bmp', 'webp'] }],
    });
  } catch (error) {
    console.error('Error selecting file:', error);
    return;
  }
  if (!selected || Array.isArray(selected)) return;
  await applyImage({ type: 'local', source: selected });
}

async function handleLoadImageFromUrl() {
  const url = imageUrlInput.value.trim();
  if (!url) {
    ElMessage.warning(t('screenEditor.enterImageUrl'));
    return;
  }
  if (await applyImage({ type: 'url', source: url })) imageUrlInput.value = '';
}

function handleRemoveImage() {
  commit({ ...localConfig.value, image: undefined });
  scheduleSend();
}

function commitTexts(texts: TextElementType[], textIds = localConfig.value.textIds) {
  commit({ ...localConfig.value, texts, textIds });
  scheduleSend();
}

function handleAddText(text: TextElementType) {
  commitTexts(
    [...localConfig.value.texts, text],
    localConfig.value.textIds.filter((id) => id !== text.id),
  );
  selectedText.value = text;
  ElMessage.success(t('screenEditor.textAdded'));
}

function handleUpdateSelectedText(text: TextElementType) {
  commitTexts(localConfig.value.texts.map((t) => (t.id === text.id ? text : t)));
  selectedText.value = text;
}

function handleRemoveText(textId: number) {
  if (selectedText.value?.id === textId) {
    selectedText.value = null;
  }

  commitTexts(
    localConfig.value.texts.filter((t) => t.id !== textId),
    [...localConfig.value.textIds, textId].sort((a, b) => a - b),
  );
}

function handleTextClick(textId: number | null) {
  if (textId === null) {
    selectedText.value = null;
    return;
  }

  const text = localConfig.value.texts.find((t) => t.id === textId);

  selectedText.value = text || null;
}

function handleUpdateTextPosition(textId: number, x: number, y: number) {
  const texts = localConfig.value.texts.map((t) =>
    t.id === textId ? { ...t, x: Math.round(x), y: Math.round(y) } : t
  );
  commitTexts(texts);
  if (selectedText.value?.id === textId) {
    selectedText.value = texts.find((t) => t.id === textId) ?? null;
  }
}

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

      <div class="section-divider" />

      <TextElement :text="selectedText" :text-ids="localConfig.textIds" @add:text="handleAddText"
        @update:text="handleUpdateSelectedText" @deselect="selectedText = null"
        @delete="(id) => handleRemoveText(id)" />
    </div>
  </section>
</template>

<style scoped>
.section-divider {
  height: 1px;
  margin: 6px 0;
  background: var(--el-border-color-lighter);
}

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

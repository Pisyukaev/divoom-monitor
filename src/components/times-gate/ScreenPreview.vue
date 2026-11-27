<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue';
import type { CSSProperties } from 'vue';
import { useI18n } from 'vue-i18n';
import { CircleCloseFilled, WarningFilled } from '@element-plus/icons-vue';
import type { ScreenConfig, TextElement } from '../../types/screen';
import { SCREEN_SIZE, renderScreen, textBoxHeight } from '../../utils/screenRender';

const props = defineProps<{
  config: ScreenConfig;
  scale?: number;
  selectedText?: TextElement | null;
  /** Read-only thumbnail: no grid, no text handles, no frame. */
  compact?: boolean;
}>();

const emit = defineEmits<{
  'update:text-position': [textId: number, x: number, y: number];
  'text-click': [textId: number | null];
  'text-delete': [textId: number];
}>();

const { t } = useI18n();

const previewSize = computed(() => props.scale || 400);
const scaleFactor = computed(() => previewSize.value / SCREEN_SIZE);

// The preview shows the exact picture that is sent to the device
// (utils/screenRender.ts); text handles are drawn on top of it.
const rendered = ref<string | null>(null);
const imageError = ref<string | null>(null);
const isEmpty = computed(() => !props.config.image && props.config.texts.length === 0);

let renderSeq = 0;
let renderTimer: ReturnType<typeof setTimeout> | undefined;

async function render() {
  const seq = ++renderSeq;
  if (isEmpty.value) {
    rendered.value = null;
    imageError.value = null;
    return;
  }
  try {
    const result = await renderScreen(props.config);
    if (seq !== renderSeq) return; // a newer render started meanwhile
    rendered.value = result.dataUrl;
    imageError.value = result.imageError;
  } catch (e) {
    if (seq === renderSeq) imageError.value = String(e);
  }
}

watch(
  () => [props.config.image, props.config.texts],
  () => {
    clearTimeout(renderTimer);
    renderTimer = setTimeout(render, 16); // coalesce drag updates into one frame
  },
  { deep: true, immediate: true },
);

const previewRef = ref<HTMLDivElement | null>(null);
const draggedTextId = ref<number | null>(null);
const dragOffset = ref({ x: 0, y: 0 });
const isDragging = ref(false);
const mouseDownPosition = ref({ x: 0, y: 0 });
const clickedTextId = ref<number | null>(null);

function handleTextMouseDown(e: MouseEvent, text: TextElement) {
  if (!previewRef.value) return;
  if ((e.target as HTMLElement).closest('.delete-icon')) return;

  mouseDownPosition.value = { x: e.clientX, y: e.clientY };
  isDragging.value = false;
  clickedTextId.value = text.id;

  const rect = previewRef.value.getBoundingClientRect();
  dragOffset.value = {
    x: e.clientX - rect.left - text.x * scaleFactor.value,
    y: e.clientY - rect.top - text.y * scaleFactor.value,
  };
  draggedTextId.value = text.id;
}

function handleMouseMove(e: MouseEvent) {
  if (draggedTextId.value === null || !previewRef.value) return;

  // More than 5px of movement counts as a drag rather than a click.
  if (Math.abs(e.clientX - mouseDownPosition.value.x) > 5 || Math.abs(e.clientY - mouseDownPosition.value.y) > 5) {
    isDragging.value = true;
  }
  if (!isDragging.value) return;

  const rect = previewRef.value.getBoundingClientRect();
  const newX = (e.clientX - rect.left - dragOffset.value.x) / scaleFactor.value;
  const newY = (e.clientY - rect.top - dragOffset.value.y) / scaleFactor.value;
  const clamp = (v: number) => Math.max(0, Math.min(SCREEN_SIZE - 1, v));
  emit('update:text-position', draggedTextId.value, clamp(newX), clamp(newY));
}

function handleMouseUp() {
  if (!isDragging.value && clickedTextId.value !== null) {
    emit('text-click', clickedTextId.value);
  }
  draggedTextId.value = null;
  clickedTextId.value = null;
  isDragging.value = false;
}

function handleDeleteText(e: MouseEvent, textId: number) {
  e.preventDefault();
  e.stopPropagation();
  emit('text-delete', textId);
}

function getTextHandleStyle(text: TextElement): CSSProperties {
  const s = scaleFactor.value;
  return {
    left: `${text.x * s}px`,
    top: `${text.y * s}px`,
    width: `${text.textWidth * s}px`,
    height: `${textBoxHeight(text) * s}px`,
  };
}

onMounted(() => {
  window.addEventListener('mousemove', handleMouseMove);
  window.addEventListener('mouseup', handleMouseUp);
});

onUnmounted(() => {
  clearTimeout(renderTimer);
  window.removeEventListener('mousemove', handleMouseMove);
  window.removeEventListener('mouseup', handleMouseUp);
});
</script>

<template>
  <div ref="previewRef" class="screen-preview" :class="{ compact }" :style="{
    width: `${previewSize}px`,
    height: `${previewSize}px`,
  }">
    <div class="preview-background" @click="emit('text-click', null)">
      <img v-if="rendered" :src="rendered" alt="Screen preview" class="preview-image" draggable="false" />
      <div v-else class="empty-background">
        <span v-if="!compact">128 × 128</span>
      </div>
      <div v-if="!compact" class="grid-overlay"></div>
    </div>

    <div v-if="imageError && !compact" class="image-error">
      <el-icon><WarningFilled /></el-icon>
      <span>{{ t('screenEditor.imageLoadError', { error: imageError }) }}</span>
    </div>

    <template v-if="!compact">
      <div v-for="text in config.texts" :key="text.id" class="text-handle"
        :class="{ 'text-handle-selected': props.selectedText?.id === text.id }" :style="getTextHandleStyle(text)"
        :title="text.content" @mousedown="handleTextMouseDown($event, text)">
        <el-icon class="delete-icon" size="22" @click="handleDeleteText($event, text.id)">
          <CircleCloseFilled />
        </el-icon>
      </div>
    </template>
  </div>
</template>

<style scoped>
.screen-preview {
  position: relative;
  border-radius: 14px;
  background-color: #000;
  overflow: hidden;
  user-select: none;
  box-shadow: 0 0 0 6px var(--app-screen-bezel), 0 12px 32px rgba(0, 0, 0, 0.25);
}

.screen-preview.compact {
  border-radius: 10px;
  box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.08);
}

.preview-background {
  position: absolute;
  width: 100%;
  height: 100%;
  background-color: #111216;
  background-image: radial-gradient(rgba(255, 255, 255, 0.07) 1px, transparent 1.3px);
  background-size: 8px 8px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.grid-overlay {
  position: absolute;
  inset: 0;
  background-image: linear-gradient(rgba(255, 255, 255, 0.1) 1px, transparent 1px),
    linear-gradient(90deg, rgba(255, 255, 255, 0.1) 1px, transparent 1px);
  background-size: 32px 32px;
  pointer-events: none;
  opacity: 0.3;
}

.preview-image {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  /* Show the 128 × 128 picture as the LCD will: sharp pixels, no blur. */
  image-rendering: pixelated;
}

.empty-background {
  color: rgba(255, 255, 255, 0.5);
  font-size: 14px;
  position: relative;
}

.image-error {
  position: absolute;
  left: 8px;
  right: 8px;
  bottom: 8px;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.75);
  color: var(--el-color-danger);
  font-size: 12px;
  z-index: 20;
}

.text-handle {
  position: absolute;
  z-index: 10;
  cursor: move;
  border-radius: 2px;
  outline: 1px dashed transparent;
}

.text-handle:hover {
  outline-color: var(--el-color-primary);
  background-color: rgba(255, 255, 255, 0.06);
}

.text-handle-selected {
  outline: 2px solid var(--el-color-primary) !important;
  background-color: rgba(255, 255, 255, 0.08);
}

.delete-icon {
  position: absolute;
  top: 0;
  right: 0;
  transform: translate(50%, -50%);
  cursor: pointer;
  color: #ff4d4f;
  opacity: 0;
}

.text-handle:hover .delete-icon,
.text-handle-selected .delete-icon {
  opacity: 1;
}
</style>

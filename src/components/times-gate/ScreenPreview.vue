<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from 'vue';
import type { CSSProperties } from 'vue';
import { readFile } from '@tauri-apps/plugin-fs';
import { CircleCloseFilled } from '@element-plus/icons-vue';
import type { ScreenConfig, TextElement } from '../../types/screen';

const props = defineProps<{
  config: ScreenConfig;
  scale?: number;
  selectedText?: TextElement | null;
}>();

const emit = defineEmits<{
  'update:text-position': [textId: number, x: number, y: number];
  'text-click': [textId: number | null];
  'text-delete': [textId: number];
}>();

const previewSize = computed(() => props.scale || 400);
const actualSize = 128;
const scaleFactor = computed(() => previewSize.value / actualSize);

const previewRef = ref<HTMLDivElement | null>(null);
const draggedTextId = ref<number | null>(null);
const dragOffset = ref({ x: 0, y: 0 });
const imageDataUrl = ref<string | null>(null);
const isDragging = ref(false);
const mouseDownPosition = ref({ x: 0, y: 0 });
const clickedTextId = ref<number | null>(null);
const textWidths = ref<Map<number, number>>(new Map());

const imageUrl = computed(() => {
  if (!props.config.image) return null;
  if (props.config.image.type === 'url') {
    return props.config.image.source;
  }
  return imageDataUrl.value;
});

async function loadLocalImage(path: string) {
  try {
    // readFile returns Uint8Array in Tauri v2
    const bytes = await readFile(path, {});
    const blob = new Blob([bytes]);
    const reader = new FileReader();
    reader.onload = () => {
      imageDataUrl.value = reader.result as string;
    };
    reader.readAsDataURL(blob);
  } catch (error) {
    console.error('Error loading local image:', error);
    imageDataUrl.value = null;
  }
}

watch(
  () => props.config.image,
  (newImage) => {
    if (newImage && newImage.type === 'local') {
      loadLocalImage(newImage.source);
    } else if (newImage && newImage.type === 'url') {
      imageDataUrl.value = null;
    } else {
      imageDataUrl.value = null;
    }
  },
  { immediate: true }
);

function handleTextMouseDown(e: MouseEvent, text: TextElement) {
  if (!previewRef.value) {
    return;
  }

  // Check if clicking on delete icon
  const target = e.target as HTMLElement;
  if (target.closest('.delete-icon')) {
    return;
  }

  mouseDownPosition.value = { x: e.clientX, y: e.clientY };
  isDragging.value = false;
  clickedTextId.value = text.id;

  const rect = previewRef.value.getBoundingClientRect();
  const textX = text.x * scaleFactor.value;
  const textY = text.y * scaleFactor.value;

  dragOffset.value = {
    x: e.clientX - rect.left - textX,
    y: e.clientY - rect.top - textY,
  };

  draggedTextId.value = text.id;
}

function handleMouseMove(e: MouseEvent) {
  if (draggedTextId.value === null || !previewRef.value) {
    return;
  }

  // Check if mouse moved significantly (more than 5px) to consider it dragging
  const deltaX = Math.abs(e.clientX - mouseDownPosition.value.x);
  const deltaY = Math.abs(e.clientY - mouseDownPosition.value.y);
  if (deltaX > 5 || deltaY > 5) {
    isDragging.value = true;
  }

  const rect = previewRef.value.getBoundingClientRect();
  const newX = (e.clientX - rect.left - dragOffset.value.x) / scaleFactor.value;
  const newY = (e.clientY - rect.top - dragOffset.value.y) / scaleFactor.value;

  // Clamp to screen bounds
  const clampedX = Math.max(0, Math.min(actualSize - 10, newX));
  const clampedY = Math.max(0, Math.min(actualSize - 10, newY));

  emit('update:text-position', draggedTextId.value, clampedX, clampedY);
}

function handleMouseUp() {
  // If it wasn't a drag, emit click event
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



function getTextElementStyle(text: TextElement): CSSProperties {
  return {
    left: `${text.x * scaleFactor.value}px`,
    top: `${text.y * scaleFactor.value}px`,
    fontSize: `${12 * scaleFactor.value}px`,
    width: `${text.textWidth * scaleFactor.value}px`,
    color: text.color || '#ffffff',
    cursor: 'move',
  };
}

function shouldAnimateText(text: TextElement): boolean {
  // Only animate if alignment is Scroll (0) AND text doesn't fit in the container
  if (text.alignment !== 0) return false;

  // Check if text width exceeds container width
  const containerWidth = text.textWidth * scaleFactor.value;
  const measuredWidth = textWidths.value.get(text.id);
  const fontSize = 12 * scaleFactor.value;
  const estimatedTextWidth = measuredWidth || (text.content.length * fontSize * 0.6);

  return estimatedTextWidth > containerWidth;
}

function needsTextDuplication(text: TextElement): boolean {
  // Only duplicate if alignment is Scroll (0) AND text doesn't fit in the container
  return shouldAnimateText(text);
}

function getTextContentStyle(text: TextElement): CSSProperties {
  const needsDuplication = needsTextDuplication(text);

  if (!needsDuplication) {
    return {
      display: 'inline-block',
      whiteSpace: 'nowrap',
    };
  }

  // Try to use measured width if available, otherwise estimate
  const measuredWidth = textWidths.value.get(text.id);
  const fontSize = 12 * scaleFactor.value;
  const textWidth = measuredWidth || (text.content.length * fontSize * 0.6);

  // Speed: 50px per second for smooth scrolling
  const duration = Math.max(3, textWidth / 50);

  return {
    display: 'inline-block',
    whiteSpace: 'nowrap',
    animation: `scroll-text ${duration}s linear infinite`,
  };
}

function measureAllTextWidths() {
  nextTick(() => {
    if (!previewRef.value) return;

    props.config.texts.forEach((text) => {
      // Measure width for all texts with Scroll alignment (0) to determine if they need animation
      if (text.alignment !== 0 || textWidths.value.has(text.id)) return;

      const textElement = previewRef.value?.querySelector(`[data-text-id="${text.id}"]`) as HTMLElement;

      // Create a temporary element to measure the width of one text instance
      const tempEl = document.createElement('span');
      tempEl.style.visibility = 'hidden';
      tempEl.style.position = 'absolute';
      tempEl.style.whiteSpace = 'nowrap';
      tempEl.style.fontSize = `${12 * scaleFactor.value}px`;
      if (textElement) {
        tempEl.style.fontFamily = getComputedStyle(textElement).fontFamily;
      }
      tempEl.textContent = text.content;
      document.body.appendChild(tempEl);

      const width = tempEl.offsetWidth;
      textWidths.value.set(text.id, width);
      document.body.removeChild(tempEl);
    });
  });
}

watch(
  () => props.config.texts,
  () => {
    // Clear widths when texts change and remeasure
    textWidths.value.clear();
    // Measure after a short delay to ensure DOM is updated
    setTimeout(() => {
      measureAllTextWidths();
    }, 100);
  },
  { deep: true }
);

watch(
  () => scaleFactor.value,
  () => {
    // Remeasure when scale changes
    textWidths.value.clear();
    setTimeout(() => {
      measureAllTextWidths();
    }, 100);
  }
);

onMounted(() => {
  measureAllTextWidths();
  window.addEventListener('mousemove', handleMouseMove);
  window.addEventListener('mouseup', handleMouseUp);
});

onUnmounted(() => {
  window.removeEventListener('mousemove', handleMouseMove);
  window.removeEventListener('mouseup', handleMouseUp);
});
</script>

<template>
  <div ref="previewRef" class="screen-preview" :style="{
    width: `${previewSize}px`,
    height: `${previewSize}px`,
  }">
    <div class="preview-background" @click="emit('text-click', null)">
      <div class="grid-overlay"></div>
      <img v-if="imageUrl" :src="imageUrl" alt="Screen preview" class="preview-image" />
      <div v-else class="empty-background">
        <span>128×128</span>
      </div>
    </div>

    <div v-for="text in config.texts" :key="text.id" class="text-element" :class="{
      'text-element-selected': props.selectedText?.id === text.id,
      'text-element-scrolling': shouldAnimateText(text)
    }" :style="getTextElementStyle(text)" @mousedown="handleTextMouseDown($event, text)">
      <el-icon class="delete-icon" @click="handleDeleteText($event, text.id)" size="24">
        <CircleCloseFilled />
      </el-icon>
      <div class="text-wrapper"
        :style="{ width: `${text.textWidth * scaleFactor}px`, overflow: 'hidden', position: 'relative' }">
        <span :data-text-id="text.id" class="text-content" :class="{ 'text-scrolling': needsTextDuplication(text) }"
          :style="getTextContentStyle(text)">
          <template v-if="needsTextDuplication(text)">
            {{ text.content }}<span class="text-separator"> </span>{{ text.content }}
          </template>
          <template v-else>
            {{ text.content }}
          </template>
        </span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.screen-preview {
  position: relative;
  border: 2px solid var(--el-border-color);
  border-radius: 8px;
  background-color: var(--el-bg-color-page);
  overflow: hidden;
  user-select: none;
}

.preview-background {
  position: absolute;
  width: 100%;
  height: 100%;
  background-color: #000;
  display: flex;
  align-items: center;
  justify-content: center;
}

.grid-overlay {
  position: absolute;
  width: 100%;
  height: 100%;
  background-image: linear-gradient(rgba(255, 255, 255, 0.1) 1px,
      transparent 1px),
    linear-gradient(90deg, rgba(255, 255, 255, 0.1) 1px, transparent 1px);
  background-size: 32px 32px;
  pointer-events: none;
  opacity: 0.3;
}

.preview-image {
  width: 100%;
  height: 100%;
  position: relative;
  z-index: 1;
}

.empty-background {
  color: rgba(255, 255, 255, 0.5);
  font-size: 14px;
  z-index: 1;
  position: relative;
}

.text-element {
  position: absolute;
  padding: 2px 4px;
  background-color: rgba(0, 0, 0, 0.5);
  border-radius: 2px;
  white-space: nowrap;
  z-index: 10;
  pointer-events: auto;
  display: flex;
  align-items: center;
  gap: 4px;
  cursor: move;
}

.text-wrapper {
  flex: 1;
  overflow: hidden;
  position: relative;
  min-width: 0;
  pointer-events: none;
}

.text-content {
  display: inline-block;
  white-space: nowrap;
  min-width: max-content;
  position: relative;
}

.text-scrolling {
  will-change: transform;
}

.text-separator {
  display: inline-block;
  width: 0.5ch;
}

.text-element-scrolling .text-content {
  will-change: transform;
}

.delete-icon {
  position: absolute;
  top: 0;
  right: 0;
  transform: translate(50%, -50%);
  cursor: pointer;
  color: #ff4d4f;
}

.delete-icon:hover {
  opacity: 1;
}

.text-element:hover {
  background-color: rgba(0, 0, 0, 0.7);
  outline: 1px dashed var(--el-color-primary);
}

.text-element:hover .delete-icon {
  opacity: 1;
}

.text-element-selected {
  outline: 2px solid var(--el-color-primary) !important;
  background-color: rgba(0, 0, 0, 0.8) !important;
}

/* Animation for scrolling text - must be outside scoped to work properly */
</style>

<style>
@keyframes scroll-text {
  0% {
    transform: translateX(0);
  }

  100% {
    transform: translateX(calc(-50% - 0.5ch));
  }
}
</style>

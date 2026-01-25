<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { readFile } from '@tauri-apps/plugin-fs';
import type { ScreenConfig, TextElement } from '../../types/screen';

const props = defineProps<{
  config: ScreenConfig;
  scale?: number;
}>();

const emit = defineEmits<{
  'update:text-position': [textId: number, x: number, y: number];
  'text-click': [textId: number];
}>();

const previewSize = computed(() => props.scale || 400);
const actualSize = 128;
const scaleFactor = computed(() => previewSize.value / actualSize);

const previewRef = ref<HTMLDivElement | null>(null);
const draggedTextId = ref<number | null>(null);
const dragOffset = ref({ x: 0, y: 0 });
const imageDataUrl = ref<string | null>(null);

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
  e.preventDefault();

  if (!previewRef.value) {
    return;
  }

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

  const rect = previewRef.value.getBoundingClientRect();
  const newX = (e.clientX - rect.left - dragOffset.value.x) / scaleFactor.value;
  const newY = (e.clientY - rect.top - dragOffset.value.y) / scaleFactor.value;

  // Clamp to screen bounds
  const clampedX = Math.max(0, Math.min(actualSize - 10, newX));
  const clampedY = Math.max(0, Math.min(actualSize - 10, newY));

  emit('update:text-position', draggedTextId.value, clampedX, clampedY);
}

function handleMouseUp() {
  draggedTextId.value = null;
}

// Add global event listeners for drag
if (typeof window !== 'undefined') {
  window.addEventListener('mousemove', handleMouseMove);
  window.addEventListener('mouseup', handleMouseUp);
}
</script>

<template>
  <div ref="previewRef" class="screen-preview" :style="{
    width: `${previewSize}px`,
    height: `${previewSize}px`,
  }">
    <div class="preview-background">
      <div class="grid-overlay"></div>
      <img v-if="imageUrl" :src="imageUrl" alt="Screen preview" class="preview-image" />
      <div v-else class="empty-background">
        <span>128×128</span>
      </div>
    </div>

    <div v-for="text in config.texts" :key="text.id" class="text-element" :style="{
      left: `${text.x * scaleFactor}px`,
      top: `${text.y * scaleFactor}px`,
      fontSize: `${(12) * scaleFactor}px`,
      color: text.color || '#ffffff',
      textAlign: text.alignment || 'left',
      cursor: 'move',
    }" @mousedown="handleTextMouseDown($event, text)">
      {{ text.content }}
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
}

.text-element:hover {
  background-color: rgba(0, 0, 0, 0.7);
  outline: 1px dashed var(--el-color-primary);
}
</style>

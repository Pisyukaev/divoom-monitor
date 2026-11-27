<script setup lang="ts">
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { Plus, Delete, Check } from '@element-plus/icons-vue';
import {
  TEXT_ALIGNMENT_OPTIONS,
  FONT_OPTIONS,
  DEFAULT_TEXT_FONT,
  DEFAULT_TEXT_ALIGNMENT,
  DEFAULT_TEXT_SIZE,
  MIN_TEXT_SIZE,
  MAX_TEXT_SIZE,
} from '../../constants';
import { SCREEN_SIZE, textAlignment, textFont, textSize } from '../../utils/screenRender';
import type { TextElement as TextElementType } from '../../types/screen';

const { t } = useI18n();

const props = defineProps<{
  /** Text selected on the preview; null = the form creates a new one. */
  text: TextElementType | null;
  /** Free TextIds for new texts. */
  textIds: number[];
}>();

const emit = defineEmits<{
  'add:text': [text: TextElementType];
  /** Every edit of the selected text, applied live. */
  'update:text': [text: TextElementType];
  deselect: [];
  delete: [textId: number];
}>();

function blankText(): TextElementType {
  return {
    id: 0,
    content: '',
    x: 0,
    y: 0,
    font: DEFAULT_TEXT_FONT,
    fontSize: DEFAULT_TEXT_SIZE,
    color: '#FFFFFF',
    alignment: DEFAULT_TEXT_ALIGNMENT,
    textWidth: SCREEN_SIZE,
  };
}

const draft = ref<TextElementType>(blankText());

const isEditing = computed(() => props.text !== null);
/** Values shown in the form; older configs' font/alignment/size are shown as they are drawn. */
const current = computed<TextElementType>(() => {
  const text = props.text ?? draft.value;
  return {
    ...text,
    font: textFont(text.font),
    fontSize: textSize(text),
    alignment: textAlignment(text.alignment),
  };
});

const canAdd = computed(() => current.value.content.trim() !== '' && props.textIds.length > 0);

function change<K extends keyof TextElementType>(key: K, value: TextElementType[K] | null | undefined) {
  if (value === null || value === undefined) return;
  const next = { ...current.value, [key]: value };
  if (isEditing.value) emit('update:text', next);
  else draft.value = next;
}

function add() {
  const id = props.textIds[0];
  if (!canAdd.value || id === undefined) return;
  emit('add:text', { ...current.value, id, content: current.value.content.trim() });
  draft.value = {
    ...blankText(),
    font: current.value.font,
    fontSize: current.value.fontSize,
    color: current.value.color,
  };
}
</script>

<template>
  <div class="text-editor">
    <div class="header">
      <span class="app-card-title">
        {{ isEditing ? t('textElement.editing') : t('textElement.newText') }}
      </span>
      <span v-if="!isEditing" class="app-muted hint">{{ t('textElement.selectHint') }}</span>
    </div>

    <div class="row">
      <el-input :model-value="current.content" :placeholder="t('textElement.textContent')" maxlength="511" clearable
        @update:model-value="(v: string) => change('content', v)" @keyup.enter="!isEditing && add()" />
      <el-color-picker :model-value="current.color" @update:model-value="(v) => change('color', v?.toUpperCase())" />
    </div>

    <div class="grid">
      <label class="field">
        <span class="app-muted label">{{ t('textElement.font') }}</span>
        <el-select :model-value="current.font" @update:model-value="(v: number) => change('font', v)">
          <el-option v-for="font in FONT_OPTIONS" :key="font.value" :label="font.label" :value="font.value"
            :style="{ fontFamily: font.family }" />
        </el-select>
      </label>
      <label class="field">
        <span class="app-muted label">{{ t('textElement.fontSize') }}</span>
        <el-input-number :model-value="current.fontSize" :min="MIN_TEXT_SIZE" :max="MAX_TEXT_SIZE"
          controls-position="right" @update:model-value="(v) => change('fontSize', v)" />
      </label>
      <label class="field">
        <span class="app-muted label">{{ t('textElement.positionX') }}</span>
        <el-input-number :model-value="current.x" :min="0" :max="SCREEN_SIZE - 1" controls-position="right"
          @update:model-value="(v) => change('x', v)" />
      </label>
      <label class="field">
        <span class="app-muted label">{{ t('textElement.positionY') }}</span>
        <el-input-number :model-value="current.y" :min="0" :max="SCREEN_SIZE - 1" controls-position="right"
          @update:model-value="(v) => change('y', v)" />
      </label>
      <label class="field wide">
        <span class="app-muted label">{{ t('textElement.alignment') }}</span>
        <el-segmented :model-value="current.alignment" :options="TEXT_ALIGNMENT_OPTIONS.map((o) => ({
          label: t(`textElement.${o.labelKey}`), value: o.value,
        }))" @change="(v: number) => change('alignment', v)" />
      </label>
      <label class="field wide">
        <span class="app-muted label">{{ t('textElement.textWidth') }}</span>
        <div class="width-row">
          <el-slider :model-value="current.textWidth" :min="16" :max="SCREEN_SIZE"
            @update:model-value="(v) => change('textWidth', Array.isArray(v) ? v[0] : v)" />
          <span class="width-value">{{ current.textWidth }}</span>
        </div>
      </label>
    </div>

    <p class="app-muted note">{{ t('textElement.drawNote') }}</p>

    <div class="actions">
      <template v-if="isEditing && text">
        <el-button :icon="Delete" type="danger" plain @click="emit('delete', text.id)">
          {{ t('textElement.delete') }}
        </el-button>
        <el-button :icon="Check" type="primary" @click="emit('deselect')">{{ t('textElement.done') }}</el-button>
      </template>
      <template v-else>
        <span v-if="textIds.length === 0" class="app-muted note">{{ t('textElement.noFreeIds') }}</span>
        <el-button :icon="Plus" type="primary" :disabled="!canAdd" @click="add">
          {{ t('textElement.addText') }}
        </el-button>
      </template>
    </div>
  </div>
</template>

<style scoped>
.text-editor {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.header {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.hint {
  font-size: 12px;
}

.row {
  display: flex;
  gap: 8px;
  align-items: center;
}

.grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px 12px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.field.wide {
  grid-column: 1 / -1;
}

.field :deep(.el-input-number),
.field :deep(.el-select),
.field :deep(.el-segmented) {
  width: 100%;
}

.width-row {
  display: flex;
  align-items: center;
  gap: 14px;
  padding-left: 8px;
}

.width-row .el-slider {
  flex: 1;
}

.width-value {
  min-width: 32px;
  text-align: right;
  font-variant-numeric: tabular-nums;
  font-weight: 600;
}

.label {
  font-size: 12px;
}

.note {
  margin: 0;
  font-size: 12px;
}

.actions {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: 8px;
}
</style>

<script setup lang="ts">
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { TEXT_ALIGNMENT_OPTIONS, FONT_OPTIONS } from '../../constants';
import type { TextElement as TextElementType } from '../../types/screen';

const { t } = useI18n();

const props = defineProps<{
  text: TextElementType | null;
  textIds: number[];
}>();

const emit = defineEmits<{
  (e: 'update:text', text: TextElementType): void;
  (e: 'submit:text', text: TextElementType): void;
  (e: 'add:text', text: TextElementType): void;
}>();

const newText = ref<TextElementType>({
  id: 0,
  content: '',
  x: 0,
  y: 0,
  font: 0,
  color: '#FFFFFF',
  alignment: 0,
  textWidth: 64,
});

const isEditing = computed(() => props.text !== null);

const currentText = computed(() => {
  return props.text ?? newText.value;
});

const availableTextIds = computed(() => [...props.textIds]);

function handleChangeTextProp<T extends keyof TextElementType,
  V extends TextElementType[T] | undefined | null>(prop: T, content: V) {
  if (content === undefined || content === null) {
    return;
  }

  if (isEditing.value) {
    emit('update:text', { ...currentText.value, [prop]: content });
  } else {
    newText.value = { ...newText.value, [prop]: content };
  }
}

function handleSubmitText() {
  if (isEditing.value) {
    emit('submit:text', currentText.value);
  } else {
    const nextId = availableTextIds.value[0];
    if (nextId === undefined) return;
    emit('add:text', { ...currentText.value, id: nextId });
  }
}


</script>
<template>
  <el-card shadow="hover" style="margin-top: 20px">
    <template #header>
      <span>{{ t('textElement.editText') }}</span>
    </template>
    <div class="control-section">
      <label class="label">{{ t('textElement.text') }}</label>
      <div class="text-container">
        <el-input v-model="currentText.content" :placeholder="t('textElement.textContent')"
          @input="(content) => handleChangeTextProp('content', content)" />

        <el-color-picker v-model="currentText.color" @change="(color) => handleChangeTextProp('color', color)" />
      </div>
      <label class="label">{{ t('textElement.fontType') }}</label>
      <el-select-v2 :options="FONT_OPTIONS" v-model="currentText.font" class="font-selector"
        @change="(font) => handleChangeTextProp('font', font)" />

      <el-radio-group v-model="currentText.alignment" class="radio-group" size="small"
        @change="(alignment) => handleChangeTextProp('alignment', alignment as number)">
        <el-radio-button v-for="option in TEXT_ALIGNMENT_OPTIONS" :key="option.value" :value="option.value">
          {{ option.label }}
        </el-radio-button>
      </el-radio-group>

      <label class="label">{{ t('textElement.textWidth') }}</label>
      <el-input-number v-model="currentText.textWidth" :min="16" :max="64" class="input-number"
        @change="(textWidth) => handleChangeTextProp('textWidth', textWidth)" />

      <div class="input-number-container">
        <label class="label">
          {{ t('textElement.positionX') }}
        </label>
        <el-input-number v-model="currentText.x" :min="0" :max="128" class="input-number"
          @change="(x) => handleChangeTextProp('x', x)" />
        <label class="label">
          {{ t('textElement.positionY') }}
        </label>
        <el-input-number v-model="currentText.y" :min="0" :max="128" class="input-number"
          @change="(y) => handleChangeTextProp('y', y)" />
      </div>

      <el-button type="success" @click="handleSubmitText" :disabled="!isEditing && availableTextIds.length === 0"
        class="button">
        {{ !isEditing ? t('textElement.addText') : t('textElement.sendToDevice') }}
      </el-button>
    </div>
  </el-card>
</template>
<style scoped>
.text-container {
  display: flex;
  flex-wrap: nowrap;
  margin-bottom: 10px;
}

.control-section {
  display: flex;
  flex-direction: column;
}

.font-selector {
  margin-bottom: 10px;
}

.input-number {
  width: 100%;
  margin-bottom: 10px;
}

.color-picker {
  margin-bottom: 10px;
}

.radio-group {
  display: flex;
  flex-direction: row;
  justify-content: center;
  align-items: center;
  flex-wrap: nowrap;
  margin-bottom: 10px;
}

.button {
  margin-top: 10px;
}

.label {
  font-size: small;
  color: var(--el-text-color-secondary);
}

.input-number-container {
  display: flex;
  flex-direction: column;
}
</style>
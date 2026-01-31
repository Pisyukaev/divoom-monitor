<script setup lang="ts">
import { ref, computed } from 'vue';
import { TEXT_ALIGNMENT_OPTIONS, FONT_OPTIONS } from '../../constants';
import type { TextElement as TextElementType } from '../../types/screen';

const props = defineProps<{
  text: TextElementType | null;
}>();

const emit = defineEmits<{
  (e: 'update:text', text: TextElementType): void;
  (e: 'submit:text', text: TextElementType): void;
  (e: 'add:text', text: TextElementType): void;
}>();

const textId = ref(0);
const newText = computed<TextElementType>(() => ({
  id: textId.value,
  content: '',
  x: 0,
  y: 0,
  font: 0,
  color: '#FFFFFF',
  alignment: 0,
  textWidth: 64,
}));

const currentText = computed(() => {
  return props.text ?? newText.value;
});


function generateTextId() {
  return textId.value++
}


function handleChangeTextProp<T extends keyof TextElementType,
  V extends TextElementType[T] | undefined | null>(prop: T, content: V) {
  if (!content) {
    return
  }
  currentText.value[prop] = content;

  if (!props.text) {
    return
  }

  emit('update:text', currentText.value);
}

function handleSubmitText() {
  if (props.text) {
    emit('submit:text', currentText.value);
  } else {
    emit('add:text', { ...currentText.value, id: generateTextId() });
  }
}


</script>
<template>
  <el-card shadow="hover" style="margin-top: 20px">
    <template #header>
      <span>Редактирование текста</span>
    </template>
    <div class="control-section">
      <label class="label">Текст</label>
      <div class="text-container">
        <el-input v-model="currentText.content" placeholder="Содержимое текста"
          @input="(content) => handleChangeTextProp('content', content)" />

        <el-color-picker v-model="currentText.color" @change="(color) => handleChangeTextProp('color', color)" />
      </div>
      <label class="label">Тип шрифта</label>
      <el-select-v2 :options="FONT_OPTIONS" v-model="currentText.font" class="font-selector"
        @change="(font) => handleChangeTextProp('font', font)" />

      <el-radio-group v-model="currentText.alignment" class="radio-group" size="small"
        @change="(alignment) => handleChangeTextProp('alignment', alignment as number)">
        <el-radio-button v-for="option in TEXT_ALIGNMENT_OPTIONS" :key="option.value" :value="option.value">
          {{ option.label }}
        </el-radio-button>
      </el-radio-group>

      <label class="label">Ширина
        текста</label>
      <el-input-number v-model="currentText.textWidth" :min="16" :max="64" class="input-number"
        @change="(textWidth) => handleChangeTextProp('textWidth', textWidth)" />

      <div class="input-number-container">
        <label class="label">
          Позиция (X)
        </label>
        <el-input-number v-model="currentText.x" :min="0" :max="128" class="input-number"
          @change="(x) => handleChangeTextProp('x', x)" />
        <label class="label">
          Позиция (Y)
        </label>
        <el-input-number v-model="currentText.y" :min="0" :max="128" class="input-number"
          @change="(y) => handleChangeTextProp('y', y)" />
      </div>

      <el-button type="success" @click="handleSubmitText" class="button">
        {{ !props.text ? 'Добавить текст' : 'Отправить на устройство' }}
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
<script setup lang="ts">
import type { Component } from 'vue';

defineProps<{
  title: string;
  description?: string;
  icon?: Component;
  /** Stack the control under the text (for wide controls like sliders). */
  stacked?: boolean;
}>();
</script>

<template>
  <div class="setting-row" :class="{ stacked }">
    <div class="setting-main">
      <span v-if="icon" class="setting-icon">
        <el-icon><component :is="icon" /></el-icon>
      </span>
      <div class="setting-text">
        <div class="setting-title">{{ title }}</div>
        <div v-if="description" class="setting-desc">{{ description }}</div>
      </div>
      <div v-if="!stacked" class="setting-control">
        <slot />
      </div>
    </div>
    <div v-if="stacked" class="setting-control-stacked">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.setting-row {
  padding: 14px 0;
}

.setting-row + .setting-row {
  border-top: 1px solid var(--el-border-color-lighter);
}

.setting-main {
  display: flex;
  align-items: center;
  gap: 14px;
}

.setting-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 34px;
  height: 34px;
  flex-shrink: 0;
  border-radius: 9px;
  background: var(--el-color-primary-light-9);
  color: var(--el-color-primary);
  font-size: 17px;
}

.setting-text {
  flex: 1;
  min-width: 0;
}

.setting-title {
  font-weight: 500;
}

.setting-desc {
  font-size: 12.5px;
  color: var(--el-text-color-secondary);
  margin-top: 1px;
}

.setting-control {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 8px;
}

.setting-control-stacked {
  margin-top: 12px;
  padding-left: 48px;
}
</style>

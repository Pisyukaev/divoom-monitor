<script setup lang="ts">
import { useTheme } from '../composables/useTheme';
import { Moon, Sunny } from '@element-plus/icons-vue';

const { isDark, toggleTheme } = useTheme();
</script>

<template>
  <el-button
    :icon="isDark ? Sunny : Moon"
    circle
    @click="toggleTheme"
    class="theme-toggle"
    :title="isDark ? 'Переключить на светлую тему' : 'Переключить на темную тему'"
  />
</template>

<style scoped>
.theme-toggle {
  position: absolute;
  right: 20px;
}
</style>


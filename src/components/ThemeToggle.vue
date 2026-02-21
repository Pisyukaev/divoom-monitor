<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { useTheme } from '../composables/useTheme';
import { Moon, Sunny } from '@element-plus/icons-vue';

const { t } = useI18n();
const { isDark, toggleTheme } = useTheme();
</script>

<template>
  <el-button
    :icon="isDark ? Sunny : Moon"
    circle
    @click="toggleTheme"
    class="theme-toggle"
    :title="isDark ? t('themeToggle.switchToLight') : t('themeToggle.switchToDark')"
  />
</template>

<style scoped>
.theme-toggle {
  flex-shrink: 0;
}
</style>


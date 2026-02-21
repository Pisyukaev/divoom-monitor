<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import SettingsLayout from '../components/SettingsLayout.vue';

const { t } = useI18n();
</script>

<template>
  <SettingsLayout>
    <router-view v-slot="{ Component }">
      <component :is="Component" v-if="Component" />
      <div v-else class="loading-placeholder">
        <p>{{ t('deviceSettings.loading') }}</p>
      </div>
    </router-view>
  </SettingsLayout>
</template>

<style scoped>
.loading-placeholder {
  padding: 20px;
  text-align: center;
}
</style>

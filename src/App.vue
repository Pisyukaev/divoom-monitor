<script setup lang="ts">
import { onMounted } from 'vue';
import { RouterView } from 'vue-router';
import { useI18n } from 'vue-i18n';
import AppSidebar from './components/layout/AppSidebar.vue';
import { sendConfigsToAllDevices } from './composables/useAutoSendConfig';
import { startPcMonitorForAllDevices } from './composables/usePcMonitorSend';

const { t } = useI18n();

onMounted(() => {
  startPcMonitorForAllDevices();
  sendConfigsToAllDevices();
});
</script>

<template>
  <div class="shell">
    <AppSidebar />
    <main class="shell-main">
      <RouterView v-slot="{ Component, route: currentRoute }">
        <component :is="Component" v-if="Component" />
        <div v-else class="no-route">
          <p>{{ t('app.routeNotFound', { path: currentRoute.path }) }}</p>
        </div>
      </RouterView>
    </main>
  </div>
</template>

<style scoped>
.shell {
  display: flex;
  height: 100vh;
  overflow: hidden;
  background: var(--el-bg-color-page);
}

.shell-main {
  flex: 1;
  min-width: 0;
  overflow-y: auto;
}

.no-route {
  padding: 40px;
  text-align: center;
}
</style>

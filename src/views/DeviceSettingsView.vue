<script setup lang="ts">
import { onMounted } from 'vue';
import { useRoute } from 'vue-router';
import SettingsLayout from '../components/SettingsLayout.vue';

const route = useRoute();

onMounted(() => {
  console.log('DeviceSettingsView mounted, route:', route.path, route.name);
});
</script>

<template>
  <SettingsLayout>
    <router-view v-slot="{ Component }">
      <component :is="Component" v-if="Component" />
      <div v-else class="loading-placeholder">
        <p>Загрузка...</p>
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

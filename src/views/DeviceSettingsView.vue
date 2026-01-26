<script setup lang="ts">
import { onMounted } from 'vue';
import { useRoute } from 'vue-router';
import DeviceSettingsLayout from '../components/DeviceSettingsLayout.vue';

const route = useRoute();

onMounted(() => {
  console.log('DeviceSettingsView mounted, route:', route.path, route.name);
});
</script>

<template>
  <DeviceSettingsLayout>
    <router-view v-slot="{ Component }">
      <component :is="Component" v-if="Component" />
      <div v-else class="loading-placeholder">
        <p>Загрузка...</p>
      </div>
    </router-view>
  </DeviceSettingsLayout>
</template>

<style scoped>
.loading-placeholder {
  padding: 20px;
  text-align: center;
}
</style>

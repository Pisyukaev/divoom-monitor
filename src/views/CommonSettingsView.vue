<script setup lang="ts">
import { computed } from 'vue';
import { useRoute } from 'vue-router';
import CommonSettings from '../components/settings/common-settings.vue';

const route = useRoute();

const deviceId = computed(() => route.params.id as string);

const deviceIp = computed(() => {
  const decodedId = decodeURIComponent(deviceId.value);
  if (decodedId.match(/^\d+\.\d+\.\d+\.\d+$/)) {
    return decodedId;
  }
  return decodedId;
});
</script>

<template>
  <div class="settings-content">
    <CommonSettings :device-id="deviceId" :device-ip="deviceIp" />
  </div>
</template>

<style scoped>
.settings-content {
  width: 100%;
  max-width: 1200px;
  margin: 0 auto;
}
</style>

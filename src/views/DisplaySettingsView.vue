<script setup lang="ts">
import { computed } from 'vue';
import { useRoute } from 'vue-router';
import ScreenSettings from '../components/settings/screen-settings.vue';

const route = useRoute();

const deviceId = computed(() => route.params.id as string);

const deviceIp = computed(() => {
  const decodedId = decodeURIComponent(deviceId.value);
  if (decodedId.match(/^\d+\.\d+\.\d+\.\d+$/)) {
    return decodedId;
  }
  return '';
});

</script>

<template>
  <div class="screens-content">
    <ScreenSettings v-if="deviceId && deviceIp" :device-id="deviceId" :device-ip="deviceIp" />
    <el-alert v-else title="Настройки экранов доступны только для устройств Times Gate" type="info" :closable="false"
      show-icon />
  </div>
</template>

<style scoped>
.screens-content {
  width: 100%;
  max-width: 1200px;
  margin: 0 auto;
}
</style>

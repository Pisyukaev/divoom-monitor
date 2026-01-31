<script setup lang="ts">
import ScreenManager from '../times-gate/ScreenManager.vue';

const props = defineProps<{
  deviceId: string;
  deviceIp: string;
}>();
</script>

<template>
  <div class="screen-settings">
    <ScreenManager :device-id="deviceId" :device-ip="deviceIp" />
  </div>
</template>

<style scoped>
.screen-settings {
  width: 100%;
}
</style>

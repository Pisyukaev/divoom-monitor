<script setup lang="ts">
import { computed } from 'vue';
import { useRoute } from 'vue-router';
import { useI18n } from 'vue-i18n';
import ScreenSettings from '../components/settings/screen-settings.vue';

const route = useRoute();
const { t } = useI18n();

const deviceId = computed(() => route.params.id as string);

const deviceIp = computed(() => {
  const decodedId = decodeURIComponent(deviceId.value);
  if (/^(\d{1,3}\.){3}\d{1,3}$/.test(decodedId)) {
    return decodedId;
  }
  return '';
});
</script>

<template>
  <div class="screens-content">
    <ScreenSettings v-if="deviceId && deviceIp" :device-id="deviceId" :device-ip="deviceIp" />
    <el-alert v-else :title="t('displaySettings.timesGateOnly')" type="info" :closable="false" show-icon />
  </div>
</template>

<style scoped>
.screens-content {
  width: 100%;
  max-width: 1200px;
  margin: 0 auto;
}
</style>

<script setup lang="ts">
import { computed } from 'vue';
import { useRoute } from 'vue-router';
import { useI18n } from 'vue-i18n';
import ScreenManager from '../components/times-gate/ScreenManager.vue';
import { parseDeviceIp } from '../utils/device';

const route = useRoute();
const { t } = useI18n();

const deviceId = computed(() => route.params.id as string);
const deviceIp = computed(() => parseDeviceIp(deviceId.value));
</script>

<template>
  <ScreenManager v-if="deviceIp" :device-id="deviceId" :device-ip="deviceIp" />
  <el-alert v-else :title="t('displaySettings.timesGateOnly')" type="info" :closable="false" show-icon />
</template>

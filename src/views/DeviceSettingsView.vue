<script setup lang="ts">
import SettingsLayout from '../components/SettingsLayout.vue';
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

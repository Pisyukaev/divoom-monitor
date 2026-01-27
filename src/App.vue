<script setup lang="ts">
import { onMounted } from 'vue';
import { RouterView, useRoute } from 'vue-router';

const route = useRoute();

onMounted(() => {
  console.log('App mounted, current route:', route.path, route.name);
});
</script>

<template>
  <div class="app-container">
    <RouterView v-slot="{ Component, route: currentRoute }">
      <component :is="Component" v-if="Component" :key="currentRoute.path" />
      <div v-else class="no-route">
        <p>Маршрут не найден: {{ currentRoute.path }}</p>
      </div>
    </RouterView>
  </div>
</template>

<style>
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

* {
  box-sizing: border-box;
}

body {
  margin: 0;
  padding: 0;
}

#app {
  width: 100%;
  min-height: 100vh;
}
</style>

<style scoped>
.app-container {
  min-height: 100vh;
  background-color: var(--el-bg-color-page);
  display: flex;
  flex-direction: column;
}

.app-container> :deep(div) {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.no-route {
  padding: 40px;
  text-align: center;
  color: var(--el-text-color-primary);
}
</style>

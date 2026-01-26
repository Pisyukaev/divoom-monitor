<script setup lang="ts">
import { onMounted } from 'vue';
import { RouterView, useRoute } from 'vue-router';
import ThemeToggle from './components/ThemeToggle.vue';

const route = useRoute();

onMounted(() => {
  console.log('App mounted, current route:', route.path, route.name);
});
</script>

<template>
  <div class="app-container">
    <header class="app-header">
      <h1>Divoom Device Monitor</h1>
      <ThemeToggle />
    </header>
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

.app-header {
  display: flex;
  justify-content: center;
  align-items: center;
  padding: 20px;
  background-color: var(--el-bg-color);
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  flex-shrink: 0;
  position: relative;
}

.app-header h1 {
  margin: 0;
  font-size: 24px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.no-route {
  padding: 40px;
  text-align: center;
  color: var(--el-text-color-primary);
}
</style>

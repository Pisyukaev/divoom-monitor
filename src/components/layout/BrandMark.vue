<script setup lang="ts">
// 3×3 "pixel" logo, a nod to Divoom's pixel-art displays.
const pixels = [1, 0, 1, 0, 1, 0, 1, 1, 1];
</script>

<template>
  <span class="brand-mark" aria-hidden="true">
    <span v-for="(on, i) in pixels" :key="i" class="px" :class="{ on }" />
  </span>
</template>

<style scoped>
.brand-mark {
  display: inline-grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 2px;
  width: 28px;
  height: 28px;
  padding: 5px;
  border-radius: 8px;
  background: linear-gradient(135deg, var(--el-color-primary), #a855f7);
  flex-shrink: 0;
}

.px {
  border-radius: 1.5px;
  background: rgba(255, 255, 255, 0.28);
}

.px.on {
  background: #fff;
}
</style>

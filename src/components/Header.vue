<script setup lang="ts">
import ThemeToggle from './ThemeToggle.vue';
</script>

<template>
    <header class="app-header">
        <h1>Divoom Device Monitor</h1>
        <div class="actions">
            <ThemeToggle />
        </div>
    </header>
</template>

<style scoped>
.app-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 20px;
    background-color: var(--el-bg-color);
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
    flex-shrink: 0;
    position: relative;
    gap: 16px;
}

.app-header h1 {
    margin: 0;
    font-size: 24px;
    font-weight: 600;
    color: var(--el-text-color-primary);
    text-align: center;
    flex: 1;
}

.actions {
    display: flex;
    align-items: center;
}
</style>

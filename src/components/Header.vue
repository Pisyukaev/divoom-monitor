<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart';
import ThemeToggle from './ThemeToggle.vue';

const autoStartEnabled = ref(false);
const autoStartLoading = ref(false);

onMounted(async () => {
    try {
        autoStartEnabled.value = await isEnabled();
    } catch (err) {
        console.error('Failed to check autostart status:', err);
    }
});

async function handleAutoStartChange(value: boolean) {
    autoStartLoading.value = true;
    try {
        if (value) {
            await enable();
        } else {
            await disable();
        }
        autoStartEnabled.value = await isEnabled();
    } catch (err) {
        console.error('Failed to toggle autostart:', err);
        autoStartEnabled.value = !value;
    } finally {
        autoStartLoading.value = false;
    }
}
</script>

<template>
    <header class="app-header">
        <h1>Divoom Device Monitor</h1>
        <div class="actions">
            <el-tooltip content="Автозагрузка при старте Windows" placement="bottom">
                <el-switch v-model="autoStartEnabled" :loading="autoStartLoading"
                    @change="handleAutoStartChange" />
            </el-tooltip>
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
    gap: 16px;
}
</style>

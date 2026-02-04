<script setup lang="ts">
import { computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import ThemeToggle from './ThemeToggle.vue';

const router = useRouter();
const route = useRoute();

const activeRoute = computed(() => {
    if (route.path.startsWith('/system')) {
        return 'system';
    }
    return 'devices';
});

const goTo = (path: string) => {
    router.push(path);
};
</script>

<template>
    <header class="app-header">
        <nav class="nav-links">
            <el-button :type="activeRoute === 'devices' ? 'primary' : 'default'" text @click="goTo('/')">
                Устройства
            </el-button>
            <el-button :type="activeRoute === 'system' ? 'primary' : 'default'" text @click="goTo('/system')">
                Система
            </el-button>
        </nav>
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

.nav-links {
    display: flex;
    gap: 8px;
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

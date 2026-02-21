<script setup lang="ts">
import { useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { Setting } from '@element-plus/icons-vue';

const router = useRouter();
const { t } = useI18n();

function openSettings() {
    router.push('/settings');
}
</script>

<template>
    <header class="app-header">
        <h1>{{ t('app.title') }}</h1>
        <div class="actions">
            <el-tooltip :content="t('header.appSettings')" placement="bottom">
                <el-button :icon="Setting" circle @click="openSettings" />
            </el-tooltip>
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

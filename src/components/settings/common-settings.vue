<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n';
import { Refresh, SwitchButton } from '@element-plus/icons-vue';

import { useDevice } from '../../composables/useDevice';
import { commands } from '../../constants';
import { invokeCommand } from '../../api/times-gate';
import type { DeviceSettings } from '../../types/device';

const { t } = useI18n();

const props = defineProps<{
    deviceId: string;
    deviceIp: string;
}>();

const { settings, isLoadingSettings, settingsError, fetchDeviceSettings } = useDevice()

onMounted(() => {
    handleUpdateSettings();
});

function createBooleanSetting<K extends keyof DeviceSettings>(
    key: K,
    trueValue: number = 1,
    falseValue: number = 0
) {
    return computed({
        get: () => (settings.value?.[key] as number) === trueValue,
        set: (value: boolean) => {
            if (settings.value) {
                (settings.value[key] as number) = value ? trueValue : falseValue;
            }
        },
    });
}


const isLightMode = createBooleanSetting('light_switch');
const isMirror = createBooleanSetting('mirror_flag');
const is24hours = createBooleanSetting('time24_flag');
const isCelsius = createBooleanSetting('temperature_mode', 0, 1);

const handleChangeOption =
    <K extends keyof DeviceSettings>(
        option: K,
        method: (typeof commands)[number]
    ) =>
        async (value: DeviceSettings[K]) => {
            if (settings.value && value !== undefined) {
                settings.value[option] = value;

                await invokeCommand(method, {
                    ipAddress: props.deviceIp,
                    value,
                });
            }
        };


function handleUpdateSettings() {
    fetchDeviceSettings(props.deviceId)
}


async function handleRebootDevice() {
    await invokeCommand('reboot_device', {
        ipAddress: props.deviceIp,
    });
}

</script>
<template>
    <el-card v-loading="isLoadingSettings" class="settings-card" shadow="hover">
        <template #header>
            <div class="card-header">
                <span>{{ t('commonSettings.currentSettings') }}</span>
                <div class="card-header-icons">
                    <el-button :icon="Refresh" :loading="isLoadingSettings" @click="handleUpdateSettings" size="small"
                        circle :title="t('commonSettings.refreshSettings')" />
                    <el-button :icon="SwitchButton" :loading="isLoadingSettings" @click="handleRebootDevice"
                        size="small" circle :title="t('commonSettings.rebootDevice')" />
                </div>
            </div>
        </template>

        <el-alert v-if="settingsError" :title="settingsError" type="error" :closable="false" show-icon
            style="margin-bottom: 20px" />

        <div v-if="settings && !isLoadingSettings">
            <el-descriptions :title="t('commonSettings.basicSettings')" :column="1" border>
                <el-descriptions-item v-if="settings.light_switch !== undefined" :label="t('commonSettings.powerToggle')">
                    <el-switch
                        @change="(value: string | number | boolean) => handleChangeOption('light_switch', 'set_switch_screen')(Number(Boolean(value)))"
                        v-model="isLightMode" />
                </el-descriptions-item>
                <el-descriptions-item v-if="settings.brightness !== undefined" :label="t('commonSettings.brightness')">
                    <div class="setting-value">
                        <div class="brightness-control">
                            <el-slider
                                @change="(value: number | number[]) => handleChangeOption('brightness', 'set_brightness')(Array.isArray(value) ? value[0] : value)"
                                class="brightness-slider" :percentage="settings.brightness"
                                v-model="settings.brightness" :range-end-label="`${settings.brightness}%`"
                                :format-tooltip="(value: number) => `${value}%`" :max="100" :min="0" :step="10" />
                            <span class="brightness-value">{{ `${settings.brightness}%` }}</span>
                        </div>
                    </div>
                </el-descriptions-item>

                <el-descriptions-item v-if="settings.mirror_flag !== undefined" :label="t('commonSettings.mirror')">
                    <el-switch
                        @change="(value: string | number | boolean) => handleChangeOption('mirror_flag', 'set_mirror_mode')(Number(Boolean(value)))"
                        v-model="isMirror" />
                </el-descriptions-item>
                <el-descriptions-item v-if="settings.temperature_mode !== undefined" :label="t('commonSettings.temperatureFormat')">
                    <el-button-group>
                        <el-button :type="isCelsius ? 'primary' : ''" @click="
                            () =>
                                handleChangeOption(
                                    'temperature_mode',
                                    'set_temperature_mode'
                                )(0)
                        ">
                            {{ t('commonSettings.celsius') }}
                        </el-button>
                        <el-button :type="!isCelsius ? 'primary' : ''" @click="
                            () =>
                                handleChangeOption(
                                    'temperature_mode',
                                    'set_temperature_mode'
                                )(1)
                        ">
                            {{ t('commonSettings.fahrenheit') }}
                        </el-button>
                    </el-button-group>
                </el-descriptions-item>
                <el-descriptions-item v-if="settings.time24_flag !== undefined" :label="t('commonSettings.timeFormat24')">
                    <el-switch
                        @change="(value: string | number | boolean) => handleChangeOption('time24_flag', 'set_24_hours_mode')(Number(Boolean(value)))"
                        v-model="is24hours" />
                </el-descriptions-item>
            </el-descriptions>
        </div>

        <el-empty v-if="!settings && !isLoadingSettings && !settingsError"
            :description="t('commonSettings.noSettings')" />
    </el-card>
</template>

<style scoped>
.settings-card {
    width: 100%;
    margin-bottom: 20px;
}

.card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
}

.card-header span {
    font-size: 16px;
    font-weight: 600;
    color: var(--el-text-color-primary);
}

.card-header-icons {
    display: flex;
    gap: 8px;
    align-items: center;
}

.setting-value {
    width: 100%;
}

.brightness-control {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    width: 100%;
}

.brightness-slider {
    flex: 1;
    min-width: 0;
}

.brightness-value {
    min-width: 50px;
    text-align: right;
    font-weight: 500;
    color: var(--el-text-color-primary);
    font-size: 14px;
}

@media (max-width: 768px) {
    .brightness-control {
        flex-direction: column;
        align-items: stretch;
        gap: 8px;
    }

    .brightness-slider {
        width: 100%;
    }

    .brightness-value {
        text-align: left;
        min-width: auto;
    }

    :deep(.el-descriptions-item__label) {
        width: auto;
    }

    .card-header {
        flex-direction: column;
        align-items: flex-start;
        gap: 12px;
    }

    .card-header-icons {
        width: 100%;
        justify-content: flex-end;
    }
}
</style>
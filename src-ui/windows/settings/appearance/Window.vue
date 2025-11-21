<template>
    <div class="settings-page">
        <h2 class="page-title">{{ t('ui_config.window_settings') }}</h2>
        <div class="content-container">
            <el-form label-width="auto">
                <el-form-item :label="t('ui_config.window_width')">
                    <el-input-number v-model="config.ui_config.window_width" placeholder="800" :min="400" :step="10"
                        :max="2000"
                        @change="(val: number) => configStore.updateConfig({ ui_config: { window_width: val } })" />
                </el-form-item>

                <el-form-item :label="t('ui_config.window_corner_radius')">
                    <el-input-number v-model="config.ui_config.window_corner_radius" placeholder="10" :min="0" :step="1"
                        :max="50"
                        @change="(val: number) => configStore.updateConfig({ ui_config: { window_corner_radius: val } })" />
                </el-form-item>

                <el-form-item :label="t('ui_config.use_windows_sys_control_radius')">
                    <el-switch v-model="config.ui_config.use_windows_sys_control_radius"
                        @change="(val: boolean) => configStore.updateConfig({ ui_config: { use_windows_sys_control_radius: val } })" />
                </el-form-item>
            </el-form>
        </div>
    </div>
</template>

<script lang="ts" setup>
import { useI18n } from 'vue-i18n';
import { useRemoteConfigStore } from '../../../stores/remote_config';
import { storeToRefs } from 'pinia';

const { t } = useI18n();
const configStore = useRemoteConfigStore()
const { config } = storeToRefs(configStore)
</script>

<style scoped>
.settings-page {
    padding: 20px;
    height: 100%;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
}

.page-title {
    margin-top: 0;
    margin-bottom: 20px;
    font-size: 20px;
    font-weight: 500;
    color: #303133;
}

.content-container {
    flex: 1;
    overflow-y: auto;
}
</style>

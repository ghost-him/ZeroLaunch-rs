<template>
    <el-tabs style="height: 100%;">
        <el-tab-pane :label="t('ui_config.search_and_result_settings')" style="height: 100%;overflow-y: auto;">
            <SearchResultSettings />
        </el-tab-pane>

        <el-tab-pane :label="t('ui_config.background_image_settings')" style="height: 100%;overflow-y: auto;">
            <BackgroundImageSettings />
        </el-tab-pane>

        <el-tab-pane :label="t('ui_config.window_settings')" style="height: 100%;overflow-y: auto;">
            <WindowSettings />
        </el-tab-pane>
    </el-tabs>
</template>

<script lang="ts" setup>
import { useI18n } from 'vue-i18n';
import SearchResultSettings from './components/SearchResultSettings.vue';
import BackgroundImageSettings from './components/BackgroundImageSettings.vue';
import WindowSettings from './components/WindowSettings.vue';

const { t } = useI18n();
</script>

<style>
.el-question-icon {
    margin-left: 8px;
}

.el-icon {
    font-size: 18px;
    color: #606266;
}
</style>
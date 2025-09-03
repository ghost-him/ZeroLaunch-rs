<template>
    <el-tabs style="height: 100%;">
        <el-tab-pane :label="t('app_config.general_settings')" style="height: 100%;overflow-y: auto;">
            <el-form label-width="auto">
                <el-divider content-position="left">{{ t('app_config.language_settings') }}</el-divider>
                <el-form-item :label="t('app_config.language')">
                    <el-select v-model="currentLanguage" @change="changeLanguage" style="width: 200px;">
                        <el-option :label="t('app_config.chinese')" value="zh"></el-option>
                        <el-option :label="t('app_config.english')" value="en"></el-option>
                    </el-select>
                    <el-tooltip class="box-item" effect="dark" :content="t('app_config.language_save_tip')">
                        <el-icon class="el-question-icon">
                            <QuestionFilled />
                        </el-icon>
                    </el-tooltip>
                </el-form-item>

                <el-divider content-position="left">{{ t('app_config.startup_instance') }}</el-divider>
                <el-form-item :label="t('app_config.auto_start')">
                    <el-switch v-model="config.app_config.is_auto_start"
                        @change="(val: boolean) => configStore.updateConfig({ app_config: { is_auto_start: val } })" />
                </el-form-item>

                <el-form-item :label="t('app_config.silent_start')">
                    <el-switch v-model="config.app_config.is_silent_start"
                        @change="(val: boolean) => configStore.updateConfig({ app_config: { is_silent_start: val } })" />
                </el-form-item>

                <el-form-item :label="t('app_config.launch_new_on_failure')">
                    <el-switch v-model="config.app_config.launch_new_on_failure"
                        @change="(val: boolean) => configStore.updateConfig({ app_config: { launch_new_on_failure: val } })" />
                </el-form-item>

                <el-divider content-position="left">{{ t('app_config.search_data') }}</el-divider>
                <el-form-item :label="t('app_config.search_result_count')">
                    <el-input-number v-model="config.app_config.search_result_count" :step="1" :precision="0" :min="1"
                        @change="(val: number) => configStore.updateConfig({ app_config: { search_result_count: val } })" />
                </el-form-item>

                <el-form-item :label="t('app_config.scroll_threshold')">
                    <el-input-number v-model="config.app_config.scroll_threshold" :step="1" :precision="0" :min="1"
                        @change="(val: number) => configStore.updateConfig({ app_config: { scroll_threshold: val } })" />
                    <el-tooltip class="box-item" effect="dark" :content="t('app_config.scroll_threshold_tooltip')">
                        <el-icon class="el-question-icon">
                            <QuestionFilled />
                        </el-icon>
                    </el-tooltip>
                </el-form-item>

                <el-form-item :label="t('app_config.auto_refresh_time')">
                    <el-input-number v-model="config.app_config.auto_refresh_time" :step="1" :precision="0" :min="1"
                        @change="(val: number) => configStore.updateConfig({ app_config: { auto_refresh_time: val } })">
                        <template #suffix>
                            <span>{{ t('app_config.minutes') }}</span>
                        </template>
                    </el-input-number>
                </el-form-item>

                <el-divider content-position="left">{{ t('app_config.window_interaction') }}</el-divider>
                <el-form-item :label="t('app_config.esc_priority_close')">
                    <el-switch v-model="config.app_config.is_esc_hide_window_priority"
                        @change="(val: boolean) => configStore.updateConfig({ app_config: { is_esc_hide_window_priority: val } })" />
                    <el-tooltip class="box-item" effect="dark" :content="t('app_config.esc_priority_tooltip')">
                        <el-icon class="el-question-icon">
                            <QuestionFilled />
                        </el-icon>
                    </el-tooltip>
                </el-form-item>

                <el-form-item :label="t('app_config.enable_drag_window')">
                    <el-switch v-model="config.app_config.is_enable_drag_window"
                        @change="(val: boolean) => configStore.updateConfig({ app_config: { is_enable_drag_window: val } })" />
                    <el-tooltip class="box-item" effect="dark" :content="t('app_config.drag_window_tooltip')">
                        <el-icon class="el-question-icon">
                            <QuestionFilled />
                        </el-icon>
                    </el-tooltip>
                </el-form-item>

                <el-form-item :label="t('app_config.wake_on_fullscreen')">
                    <el-switch v-model="config.app_config.is_wake_on_fullscreen"
                        @change="(val: boolean) => configStore.updateConfig({ app_config: { is_wake_on_fullscreen: val } })" />
                    <el-tooltip class="box-item" effect="dark" :content="t('app_config.wake_fullscreen_tooltip')">
                        <el-icon class="el-question-icon">
                            <QuestionFilled />
                        </el-icon>
                    </el-tooltip>
                </el-form-item>

                <el-form-item :label="t('app_config.follow_mouse')">
                    <el-switch v-model="config.app_config.show_pos_follow_mouse"
                        @change="(val: boolean) => configStore.updateConfig({ app_config: { show_pos_follow_mouse: val } })" />
                    <el-tooltip class="box-item" effect="dark" :content="t('app_config.follow_mouse_tooltip')">
                        <el-icon class="el-question-icon">
                            <QuestionFilled />
                        </el-icon>
                    </el-tooltip>
                </el-form-item>

                <el-form-item :label="t('app_config.space_is_enter')">
                    <el-switch v-model="config.app_config.space_is_enter"
                        @change="(val: boolean) => configStore.updateConfig({ app_config: { space_is_enter: val } })" />
                    <el-tooltip class="box-item" effect="dark" :content="t('app_config.space_enter_tooltip')">
                        <el-icon class="el-question-icon">
                            <QuestionFilled />
                        </el-icon>
                    </el-tooltip>
                </el-form-item>

                <el-divider content-position="left">{{ t('app_config.advanced') }}</el-divider>
                <el-form-item :label="t('app_config.debug_mode')">
                    <el-switch v-model="config.app_config.is_debug_mode"
                        @change="(val: boolean) => configStore.updateConfig({ app_config: { is_debug_mode: val } })" />
                </el-form-item>

                <el-form-item :label="t('app_config.log_level')">
                    <el-select v-model="config.app_config.log_level"
                        @change="(val: 'debug' | 'info' | 'warn' | 'error') => configStore.updateConfig({ app_config: { log_level: val } })"
                        style="width: 120px">
                        <el-option label="Debug" value="debug" />
                        <el-option label="Info" value="info" />
                        <el-option label="Warn" value="warn" />
                        <el-option label="Error" value="error" />
                    </el-select>
                    <el-tooltip placement="top" :content="t('app_config.log_level_tooltip')">
                        <el-icon class="el-question-icon">
                            <QuestionFilled />
                        </el-icon>
                    </el-tooltip>
                </el-form-item>
            </el-form>
        </el-tab-pane>

        <!-- 这里可以根据需要添加更多的 el-tab-pane 来组织其他设置 -->

    </el-tabs>
</template>

<script lang="ts" setup>

import { QuestionFilled } from '@element-plus/icons-vue'
import { useRemoteConfigStore } from '../stores/remote_config'; // 确认路径正确
import { storeToRefs } from 'pinia';
import { useI18n } from 'vue-i18n';
import { computed } from 'vue';
import { initializeLanguage } from '../i18n/index';

const { t } = useI18n();
const configStore = useRemoteConfigStore()
const { config } = storeToRefs(configStore)

// 语言切换功能 - 使用computed确保响应式更新
const currentLanguage = computed({
    get: () => config.value.app_config.language,
    set: (value: string) => {
        // 使用全局语言初始化函数
        initializeLanguage(value);
        configStore.updateConfig({ app_config: { language: value } });
    }
});

const changeLanguage = (lang: string) => {
    currentLanguage.value = lang;
};

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
<template>
    <div class="welcome-page">
        <el-card class="welcome-card">
            <!-- 1. 头部信息 -->
            <template #header>
                <div class="card-header">
                    <el-icon :size="40" color="#409EFC">
                        <Star />
                    </el-icon>
                    <div class="header-text">
                        <h1 class="title">{{ t('welcome.title') }}</h1>
                        <p class="subtitle">{{ t('welcome.subtitle') }}</p>
                    </div>
                </div>
            </template>

            <!-- 2. 快速上手 -->
            <div class="section">
                <h2 class="section-title">
                    <el-icon>
                        <InfoFilled />
                    </el-icon>
                    <span>{{ t('welcome.quick_start') }}</span>
                </h2>
                <el-steps :active="4" align-center finish-status="success">
                    <el-step v-for="step in steps" :key="step.id" :title="step.title" :description="step.description" />
                </el-steps>
            </div>

            <el-divider />

            <!-- 3. 快捷键指南 -->
            <div class="section">
                <h2 class="section-title">
                    <el-icon>
                        <Edit />
                    </el-icon>
                    <span>{{ t('welcome.shortcut_guide') }}</span>
                </h2>
                <div class="shortcuts-grid">
                    <div v-for="shortcut in shortcuts" :key="shortcut.id" class="shortcut-item">
                        <div class="shortcut-keys">
                            <el-tag v-for="key in shortcut.keys" :key="key" type="info" size="small">{{ key }}</el-tag>
                        </div>
                        <span class="shortcut-description">{{ shortcut.description }}</span>
                    </div>
                </div>
            </div>

            <el-divider />

            <!-- 4. 底部操作 -->
            <div class="footer-actions">
                <el-button type="primary" size="large" :icon="Link" @click="startUsing">
                    {{ t('welcome.visit_website') }}
                </el-button>
                <el-button size="large" :icon="Setting" @click="openSettings">
                    {{ t('welcome.open_settings') }}
                </el-button>
            </div>
        </el-card>
    </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { computed, onMounted } from 'vue';
import { open } from '@tauri-apps/plugin-shell';
import { invoke } from '@tauri-apps/api/core';
import { QuestionFilled, Star, InfoFilled, Edit, Link, Setting } from '@element-plus/icons-vue'
import { initializeLanguage } from '../i18n/index';
import { useRemoteConfigStore } from '../stores/remote_config';

const { t } = useI18n();
const configStore = useRemoteConfigStore();

// 在组件挂载时初始化语言设置
onMounted(async () => {
    try {
        // 加载配置
        await configStore.loadConfig();
        const language = configStore.config.app_config.language;
        if (language) {
            await initializeLanguage(language);

        }
    } catch (error) {
        console.warn('Welcome页面语言初始化失败:', error);
        // 使用默认语言
        await initializeLanguage('zh');
    }
});

// 快捷键数据（使用计算属性，确保语言切换时自动更新）
const shortcuts = computed(() => [
    { id: 1, keys: ['Alt', 'Space'], description: t('welcome.shortcuts.toggle_window') },
    { id: 2, keys: ['Enter'], description: t('welcome.shortcuts.launch_selected') },
    { id: 3, keys: ['Alt'], description: t('welcome.shortcuts.sort_by_recent') },
    { id: 4, keys: ['Ctrl', 'Enter'], description: t('welcome.shortcuts.launch_as_admin') },
    { id: 5, keys: ['Esc'], description: t('welcome.shortcuts.clear_or_hide') },
    { id: 6, keys: ['Shift', 'Enter'], description: t('welcome.shortcuts.bring_to_front') },
    { id: 7, keys: ['↑', '↓'], description: t('welcome.shortcuts.move_selection') },
    { id: 8, keys: ['Ctrl', 'J/K'], description: t('welcome.shortcuts.move_selection_vim') },
]);

// 步骤数据（使用计算属性，确保语言切换时自动更新）
const steps = computed(() => [
    { id: 1, title: t('welcome.steps.wake_app.title'), description: t('welcome.steps.wake_app.description') },
    { id: 2, title: t('welcome.steps.quick_search.title'), description: t('welcome.steps.quick_search.description') },
    { id: 3, title: t('welcome.steps.one_click_launch.title'), description: t('welcome.steps.one_click_launch.description') },
    { id: 4, title: t('welcome.steps.personalize.title'), description: t('welcome.steps.personalize.description') },
]);

// 方法
const startUsing = async () => {
    await open('https://zerolaunch.ghost-him.com').catch(err => {
        console.error(t('welcome.errors.cannot_open_website'), err);
    });
};

const openSettings = async () => {
    await invoke('show_setting_window').catch(error => {
        console.error(t('welcome.errors.cannot_open_settings'), error);
    });
};
</script>

<style scoped>
.welcome-page {
    display: flex;
    justify-content: center;
    height: 100%;
    background-color: #f2f3f5;
    /* Element Plus 经典的背景色 */
    padding: 20px;
    overflow-y: auto;
    box-sizing: border-box;
}

.welcome-card {
    max-width: 800px;
    width: 100%;
    margin: auto 0;
}

.card-header {
    display: flex;
    align-items: center;
    gap: 20px;
}

.header-text .title {
    font-size: 24px;
    font-weight: 600;
    color: var(--el-text-color-primary);
    margin: 0 0 8px 0;
}

.header-text .subtitle {
    font-size: 14px;
    color: var(--el-text-color-secondary);
    margin: 0;
}

.section {
    margin-top: 20px;
}

.section-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 18px;
    font-weight: 600;
    color: var(--el-text-color-primary);
    margin-bottom: 24px;
}

/* 快捷键网格布局 */
.shortcuts-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 16px;
}

.shortcut-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px;
    background-color: var(--el-fill-color-light);
    border-radius: 8px;
    border: 1px solid var(--el-border-color-lighter);
}

.shortcut-keys {
    display: flex;
    gap: 6px;
}

.shortcut-keys .el-tag {
    min-width: 30px;
    justify-content: center;
}

.shortcut-description {
    font-size: 14px;
    color: var(--el-text-color-regular);
    margin-left: 16px;
    flex-shrink: 0;
}

/* 底部按钮 */
.footer-actions {
    display: flex;
    justify-content: center;
    gap: 20px;
    margin-top: 30px;
}

/* 针对 el-steps 的微调 */
:deep(.el-step__title) {
    font-weight: 600;
}

:deep(.el-step__description) {
    font-size: 13px;
}

.el-question-icon {
    margin-left: 8px;
}
</style>

<template>
    <div class="welcome-page">
        <el-card class="welcome-card">
            <!-- 1. 头部信息 -->
            <template #header>
                <div class="card-header">
                    <el-icon :size="40" color="#409EFC">
                        <Rocket />
                    </el-icon>
                    <div class="header-text">
                        <h1 class="title">欢迎使用 ZeroLaunch</h1>
                        <p class="subtitle">一个可以容忍错别字的 Windows 应用启动器</p>
                    </div>
                </div>
            </template>

            <!-- 2. 快速上手 -->
            <div class="section">
                <h2 class="section-title">
                    <el-icon>
                        <Guide />
                    </el-icon>
                    <span>快速上手</span>
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
                        <Keyboard />
                    </el-icon>
                    <span>快捷键速查</span>
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
                    访问官网
                </el-button>
                <el-button size="large" :icon="Setting" @click="openSettings">
                    打开设置
                </el-button>
            </div>
        </el-card>
    </div>
</template>

<script setup>
// 使用 Vue 3 <script setup> 语法，更简洁
import { ref } from 'vue';
import { open } from '@tauri-apps/plugin-shell';
import { invoke } from '@tauri-apps/api/core';
import { QuestionFilled } from '@element-plus/icons-vue'
// 快捷键数据（优化了描述）
const shortcuts = ref([
    { id: 1, keys: ['Alt', 'Space'], description: '唤醒 / 隐藏主窗口' },
    { id: 2, keys: ['Enter'], description: '启动选中项' },
    { id: 3, keys: ['Ctrl', 'Enter'], description: '以管理员权限启动' },
    { id: 4, keys: ['Esc'], description: '清空输入或隐藏窗口' },
    { id: 5, keys: ['Shift', 'Space'], description: '置顶当前已打开的窗口' },
    { id: 6, keys: ['Alt'], description: '临时按最近使用时间排序' },
    { id: 7, keys: ['↑', '↓'], description: '上 / 下移动选择' },
    { id: 8, keys: ['Ctrl', 'J/K'], description: '上 / 下移动选择 (Vim模式)' },


]);

// 步骤数据（优化了描述）
const steps = ref([
    { id: 1, title: '唤醒应用', description: '按下 Alt + Space 随时唤醒搜索框' },
    { id: 2, title: '快速搜索', description: '输入应用名、拼音或缩写，高效匹配' },
    { id: 3, title: '一键启动', description: '使用方向键选择，按下 Enter 即可启动' },
    { id: 4, title: '个性配置', description: '在设置中打造你的专属启动器' },
]);

// 方法
const startUsing = () => {
    open('https://zerolaunch.ghost-him.com').catch(err => {
        console.error('无法打开官网链接:', err);
        // 可以在这里加一个 ElMessage 提示用户
    });
};

const openSettings = () => {
    invoke('show_setting_window').catch(error => {
        console.error('无法打开设置窗口:', error);
        // 可以在这里加一个 ElMessage 提示用户
    });
};
</script>

<style scoped>
.welcome-page {
    display: flex;
    justify-content: center;
    align-items: center;
    min-height: 100vh;
    background-color: #f2f3f5;
    /* Element Plus 经典的背景色 */
    padding: 20px;
    box-sizing: border-box;
}

.welcome-card {
    max-width: 800px;
    width: 100%;
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
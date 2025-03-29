<template>
    <div class="shortcut-settings" style="height: 100%; overflow-y: auto;">
        <div class="shortcut-header">
            <h2 class="shortcut-title">快捷键设置</h2>
            <div class="shortcut-actions">
                <el-button type="primary" @click="edit_shortcut_config" :disabled="is_saving">
                    <el-icon>
                        <Edit v-if="!is_editing" />
                        <Close v-else />
                    </el-icon>
                    {{ is_editing ? '取消编辑' : '开始设置' }}
                </el-button>
                <el-button type="success" @click="save_shortcut_config" :loading="is_saving" :disabled="!is_editing">
                    <el-icon>
                        <Check />
                    </el-icon>
                    保存设置
                </el-button>
                <el-button type="warning" @click="resetAllShortcuts" :disabled="!is_editing">
                    <el-icon>
                        <RefreshLeft />
                    </el-icon>
                    重置全部
                </el-button>
            </div>
        </div>

        <el-divider content-position="left">系统快捷键</el-divider>
        <div class="shortcut-settings-container">
            <el-form label-position="top" class="shortcut-form">
                <el-form-item v-for="item in shortcutItems" :key="item.key" class="shortcut-form-item">
                    <div class="shortcut-item-header">
                        <div class="shortcut-label">
                            <el-icon>
                                <component :is="item.icon" />
                            </el-icon>
                            <span>{{ item.label }}</span>
                        </div>
                        <el-tooltip :content="item.tooltip" placement="top" effect="light">
                            <el-icon class="info-icon">
                                <InfoFilled />
                            </el-icon>
                        </el-tooltip>
                    </div>

                    <div class="shortcut-item-content">
                        <ShortcutInput v-model="dirty_shortcut_config[item.key]" :disabled="!is_editing"
                            :placeholder="item.placeholder">
                        </ShortcutInput>

                        <el-button class="reset-button" :disabled="!is_editing" @click="resetShortcut(item.key)">
                            <el-icon>
                                <RefreshRight />
                            </el-icon>
                            重置
                        </el-button>
                    </div>
                </el-form-item>
            </el-form>
        </div>
    </div>
</template>

<script lang="ts" setup>
import { default_shortcut_config, ShortcutConfig } from '../api/remote_config_types';
import ShortcutInput from '../utils/ShortcutInput.vue';
import { onUnmounted, ref, markRaw } from 'vue';
import { onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useRemoteConfigStore } from '../stores/remote_config';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
    Search, ArrowLeft, ArrowRight, ArrowUp, ArrowDown,
    InfoFilled, Edit, Close, Check, RefreshLeft,
    RefreshRight
} from '@element-plus/icons-vue';
import type { Component } from 'vue'
const configStore = useRemoteConfigStore();
const is_editing = ref<boolean>(false);
const is_saving = ref<boolean>(false);
const d_shortcut_config: ShortcutConfig = default_shortcut_config();
const dirty_shortcut_config = ref<ShortcutConfig>({ ...configStore.config.shortcut_config });

// 定义快捷键标签和描述
type ShortcutKey = keyof ShortcutConfig;
const shortcutItems = ref<Array<{
    key: keyof ShortcutConfig;
    icon: Component;
    label: string;
    tooltip: string;
    placeholder: string;
}>>([
    {
        key: 'open_search_bar',
        icon: markRaw(Search),
        label: '打开搜索栏',
        tooltip: '打开搜索窗口的快捷键',
        placeholder: '例如: Ctrl+Space'
    },
    {
        key: 'arrow_left',
        icon: markRaw(ArrowLeft),
        label: '方向键左',
        tooltip: '向左移动的快捷键',
        placeholder: '例如: Left'
    },
    {
        key: 'arrow_right',
        icon: markRaw(ArrowRight),
        label: '方向键右',
        tooltip: '向右移动的快捷键',
        placeholder: '例如: Right'
    },
    {
        key: 'arrow_up',
        icon: markRaw(ArrowUp),
        label: '方向键上',
        tooltip: '向上移动的快捷键',
        placeholder: '例如: Up'
    },
    {
        key: 'arrow_down',
        icon: markRaw(ArrowDown),
        label: '方向键下',
        tooltip: '向下移动的快捷键',
        placeholder: '例如: Down'
    }
]);

const edit_shortcut_config = async () => {
    if (is_editing.value) {
        // 取消编辑，恢复原始配置
        dirty_shortcut_config.value = { ...configStore.config.shortcut_config };
        is_editing.value = false;
        try {
            await invoke('command_register_all_shortcut');
            ElMessage.info("已取消编辑并恢复快捷键");
        } catch (error) {
            handleError("快捷键恢复失败:" + error);
        }
    } else {
        try {
            await invoke('command_unregister_all_shortcut');
            is_editing.value = true;
            ElMessage.success({
                message: "已进入编辑模式，可以设置快捷键",
                duration: 2000
            });
        } catch (error) {
            handleError("快捷键解绑失败:" + error);
        }
    }
}

const save_shortcut_config = async () => {
    is_saving.value = true;
    try {
        await configStore.updateConfig({ shortcut_config: dirty_shortcut_config.value });
        await configStore.syncConfig();
        is_editing.value = false;
        ElMessage.success({
            message: "快捷键设置已保存并生效",
            type: 'success',
            duration: 2000
        });
    } catch (error) {
        handleError("快捷键保存失败: " + error);
        try {
            await invoke('command_register_all_shortcut');
        } catch (e) {
            handleError("恢复默认配置失败: " + e);
        }
    } finally {
        is_saving.value = false;
    }
}

const resetShortcut = (key: ShortcutKey) => {
    dirty_shortcut_config.value[key] = d_shortcut_config[key];
    ElMessage.info(`已重置为默认值`);
}

const resetAllShortcuts = async () => {
    try {
        await ElMessageBox.confirm(
            '确定要将所有快捷键重置为默认值吗？',
            '重置确认',
            {
                confirmButtonText: '确定重置',
                cancelButtonText: '取消',
                type: 'warning',
            }
        );
        dirty_shortcut_config.value = { ...d_shortcut_config };
        ElMessage.success("已重置所有快捷键为默认值");
    } catch {
        // 用户取消操作
    }
}

const handleError = (error: string) => {
    ElMessage({
        showClose: true,
        message: error,
        type: 'error',
        duration: 5000
    });
}

onMounted(async () => {
    // 初始化逻辑
})

onUnmounted(async () => {
    if (is_editing.value) {
        await invoke('command_register_all_shortcut');
        is_editing.value = false;
    }
})
</script>

<style scoped>
.shortcut-settings {
    max-width: 800px;
    margin: 0 auto;
    padding: 20px;
}

.shortcut-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
}

.shortcut-title {
    font-size: 22px;
    color: #303133;
    margin: 0;
}

.shortcut-actions {
    display: flex;
    gap: 12px;
}

.shortcut-settings-container {
    margin-top: 20px;
}

.shortcut-form {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
    gap: 20px;
}

.shortcut-form-item {
    background-color: var(--el-bg-color);
    padding: 16px;
    border-radius: 8px;
    box-shadow: 0 2px 12px 0 rgba(0, 0, 0, 0.05);
    transition: all 0.3s;
    margin: 0;
}

.shortcut-form-item:hover {
    box-shadow: 0 4px 16px 0 rgba(0, 0, 0, 0.1);
}

.shortcut-item-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
}

.shortcut-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 500;
    color: var(--el-text-color-primary);
}

.info-icon {
    color: var(--el-color-info);
    cursor: pointer;
    font-size: 16px;
}

.shortcut-item-content {
    display: flex;
    align-items: center;
    gap: 12px;
}

.reset-button {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--el-color-primary);
    transition: all 0.2s;
}

.reset-button:hover:not(:disabled) {
    color: var(--el-color-primary-light-3);
    background-color: var(--el-color-primary-light-9);
}

.reset-button:disabled {
    color: var(--el-text-color-disabled);
    cursor: not-allowed;
}

:deep(.el-form-item__content) {
    display: flex;
    flex-direction: column;
    align-items: stretch;
}

:deep(.el-divider__text) {
    font-size: 16px;
    font-weight: 500;
    color: var(--el-text-color-secondary);
}
</style>
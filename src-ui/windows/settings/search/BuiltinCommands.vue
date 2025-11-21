<template>
    <div class="settings-page">
        <div class="header">
            <h3>{{ t('settings.builtin_command_title') }}</h3>
            <p class="description">{{ t('settings.builtin_command_description') }}</p>
        </div>

        <div class="content-container">
            <div class="action-buttons">
                <el-button size="small" @click="enableAll">{{ t('settings.enable_all') }}</el-button>
                <el-button size="small" @click="disableAll">{{ t('settings.disable_all') }}</el-button>
            </div>

            <el-table :data="builtinCommandList" border stripe style="width: 100%">
                <el-table-column :label="t('settings.builtin_command_function')" width="150">
                    <template #default="{ row }">
                        <span class="command-description">{{ t(`builtin.${getCommandI18nKey(row.type)}_desc`) }}</span>
                    </template>
                </el-table-column>

                <el-table-column :label="t('settings.builtin_command_display_name')" width="150">
                    <template #default="{ row }">
                        <span>{{ t(`builtin.${getCommandI18nKey(row.type)}`) }}</span>
                    </template>
                </el-table-column>

                <el-table-column :label="t('settings.builtin_command_keywords')" min-width="300">
                    <template #default="{ row }">
                        <el-tag
                            v-for="(keyword, index) in row.keywords"
                            :key="index"
                            closable
                            @close="removeKeyword(row.type, index)"
                            style="margin-right: 5px; margin-bottom: 5px;"
                        >
                            {{ keyword }}
                        </el-tag>
                        <el-input
                            v-if="row.inputVisible"
                            ref="inputRef"
                            v-model="row.inputValue"
                            class="keyword-input"
                            size="small"
                            @keyup.enter="confirmInput(row)"
                            @blur="confirmInput(row)"
                        />
                        <el-button
                            v-else
                            size="small"
                            @click="showInput(row)"
                        >
                            + {{ t('settings.add_keyword') }}
                        </el-button>
                        <el-button
                            size="small"
                            @click="resetKeywords(row.type)"
                            style="margin-left: 10px;"
                        >
                            {{ t('settings.reset_to_default') }}
                        </el-button>
                    </template>
                </el-table-column>

                <el-table-column :label="t('settings.builtin_command_enabled')" width="100" align="center">
                    <template #default="{ row }">
                        <el-switch
                            v-model="row.enabled"
                            @change="updateCommandState(row.type, row.enabled)"
                        />
                    </template>
                </el-table-column>
            </el-table>
        </div>
    </div>
</template>

<script setup lang="ts">
import { computed, ref, nextTick } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElButton, ElSwitch, ElTable, ElTableColumn, ElTag, ElInput } from 'element-plus';
import { useRemoteConfigStore } from '../../../stores/remote_config';
import { storeToRefs } from 'pinia';
import type { BuiltinCommandType } from '../../../api/remote_config_types';

const { t } = useI18n();
const configStore = useRemoteConfigStore();
const { config } = storeToRefs(configStore);

interface BuiltinCommandItem {
    type: BuiltinCommandType;
    enabled: boolean;
    keywords: string[];
    inputVisible: boolean;
    inputValue: string;
}

// 将 PascalCase 转换为 snake_case (OpenSettings -> open_settings)
const getCommandI18nKey = (type: BuiltinCommandType): string => {
    return type
        .replace(/([A-Z])/g, '_$1')
        .toLowerCase()
        .slice(1);
};

const inputRef = ref();

// 从 pinia store 计算命令列表（完全从后端数据驱动）
const builtinCommandList = computed<BuiltinCommandItem[]>(() => {
    const enabledCommands = config.value.program_manager_config.loader.enabled_builtin_commands;
    const keywords = config.value.program_manager_config.loader.builtin_command_keywords;

    // 基于后端配置生成命令列表
    return Object.keys(enabledCommands).map(type => ({
        type: type as BuiltinCommandType,
        enabled: enabledCommands[type as BuiltinCommandType] ?? true,
        keywords: [...(keywords[type as BuiltinCommandType] || [])],
        inputVisible: false,
        inputValue: ''
    }));
});

// 更新单个命令的启用状态
const updateCommandState = (type: BuiltinCommandType, enabled: boolean) => {
    configStore.updateConfig({
        program_manager_config: {
            loader: {
                enabled_builtin_commands: {
                    ...config.value.program_manager_config.loader.enabled_builtin_commands,
                    [type]: enabled
                }
            }
        }
    });
};

// 更新关键词
const updateKeywords = (type: BuiltinCommandType, keywords: string[]) => {
    configStore.updateConfig({
        program_manager_config: {
            loader: {
                builtin_command_keywords: {
                    ...config.value.program_manager_config.loader.builtin_command_keywords,
                    [type]: keywords
                }
            }
        }
    });
};

// 删除关键词
const removeKeyword = (type: BuiltinCommandType, index: number) => {
    const currentKeywords = [...(config.value.program_manager_config.loader.builtin_command_keywords[type] || [])];
    currentKeywords.splice(index, 1);
    updateKeywords(type, currentKeywords);
};

// 显示输入框
const showInput = (row: BuiltinCommandItem) => {
    row.inputVisible = true;
    nextTick(() => {
        inputRef.value?.focus();
    });
};

// 确认输入
const confirmInput = (row: BuiltinCommandItem) => {
    if (row.inputValue && row.inputValue.trim()) {
        const currentKeywords = [...(config.value.program_manager_config.loader.builtin_command_keywords[row.type] || [])];
        currentKeywords.push(row.inputValue.trim());
        updateKeywords(row.type, currentKeywords);
    }
    row.inputVisible = false;
    row.inputValue = '';
};

// 重置为默认关键词（后端存储的默认值）
const resetKeywords = (type: BuiltinCommandType) => {
    // 重置为空数组，让后端使用默认值
    updateKeywords(type, []);
};

// 全部启用
const enableAll = () => {
    const updatedCommands = { ...config.value.program_manager_config.loader.enabled_builtin_commands };
    Object.keys(updatedCommands).forEach(key => {
        updatedCommands[key as BuiltinCommandType] = true;
    });
    configStore.updateConfig({
        program_manager_config: {
            loader: {
                enabled_builtin_commands: updatedCommands
            }
        }
    });
};

// 全部禁用
const disableAll = () => {
    const updatedCommands = { ...config.value.program_manager_config.loader.enabled_builtin_commands };
    Object.keys(updatedCommands).forEach(key => {
        updatedCommands[key as BuiltinCommandType] = false;
    });
    configStore.updateConfig({
        program_manager_config: {
            loader: {
                enabled_builtin_commands: updatedCommands
            }
        }
    });
};
</script>

<style scoped>
.settings-page {
    padding: 20px;
    height: 100%;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
}

.content-container {
    flex: 1;
    overflow-y: auto;
}

.header {
    margin-bottom: 20px;
}

.header h3 {
    margin: 0 0 10px 0;
    font-size: 18px;
    font-weight: 500;
    color: #303133;
}

.description {
    margin: 0;
    font-size: 14px;
    color: #909399;
}

.action-buttons {
    display: flex;
    gap: 10px;
    margin-bottom: 20px;
}

.command-description {
    font-size: 13px;
    color: #606266;
}

.keyword-input {
    width: 120px;
    margin-right: 5px;
}
</style>

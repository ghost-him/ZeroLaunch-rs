<template>
    <div class="settings-container">
        <div class="sidebar">
            <div class="header">
                <img src="../assets/icon.svg" alt="Logo" class="logo">
                <span class="title">{{ t('settings.title') }}</span>
            </div>
            <div class="menu-container">
                <div v-for="(item, index) in menuItems" :key="index" class="menu-item"
                    :class="{ active: activeIndex === index }" @click="activeIndex = index">
                    <el-icon>
                        <component :is="item.icon"></component>
                    </el-icon>
                    <span class="menu-text">{{ item.title }}</span>
                </div>

                <div v-if="config.app_config.is_debug_mode" class="menu-item" :class="{ active: activeIndex === 999 }"
                    @click="activeIndex = 999">
                    <el-icon>
                        <Monitor />
                    </el-icon>
                    <span class="menu-text">{{ t('settings.debug_mode') }}</span>
                </div>
            </div>
            <div class="footer-item">
                <el-button type="primary" @click="save_config" :disabled="activeIndex >= 5">
                    <span>{{ t('settings.save_config') }}</span>
                </el-button>
            </div>
        </div>

        <!-- 内容区域 -->
        <div class="content">
            <!-- 常规设置 -->
            <section v-if="activeIndex === 0" class="page" style="height: 100%;overflow-y: auto;">
                <AppConfigSetting></AppConfigSetting>
            </section>

            <section v-if="activeIndex === 1" class="page">
                <UIConfigSetting></UIConfigSetting>
            </section>

            <!-- 外观设置 -->
            <section v-if="activeIndex === 2" class="page">
                <ProgramIndex></ProgramIndex>
            </section>

            <section v-if="activeIndex === 3" class="page">
                <el-tabs style="height: 100%">
                    <el-tab-pane :label="t('settings.custom_web_search')" style="height: 100%">
                        <div style="display: flex; flex-direction: column; height: 100%;">
                            <el-button class="mt-4" style="width: 100%;  flex-shrink: 0;" @click="addIndexWebPage">
                                {{ t('settings.add_item') }}
                            </el-button>
                            <el-table :data="index_web_pages" stripe
                                style="width: 100%;flex-grow: 1; height: 0; min-height: 0; margin-top: 10px;">
                                <el-table-column :label="t('settings.keyword_for_search')" show-overflow-tooltip
                                    fixed="left" width="150">
                                    <template #default="scope">
                                        <el-input v-model="index_web_pages[scope.$index][0]" size="small"
                                            :placeholder="t('settings.enter_keyword')"
                                            @change="updateIndexWebPages"></el-input>
                                    </template>
                                </el-table-column>
                                <el-table-column :label="t('settings.target_website_address')" show-overflow-tooltip>
                                    <template #default="scope">
                                        <el-input v-model="index_web_pages[scope.$index][1]" size="small"
                                            :placeholder="t('settings.enter_target_path')"
                                            @change="updateIndexWebPages"></el-input>
                                    </template>
                                </el-table-column>
                                <el-table-column fixed="right" :label="t('settings.actions')" width="100">
                                    <template #default="scope">
                                        <el-button link size="small" type="danger"
                                            @click="deleteIndexWebPages(scope.$index)">
                                            {{ t('settings.delete_row') }}
                                        </el-button>
                                    </template>
                                </el-table-column>
                            </el-table>
                        </div>
                    </el-tab-pane>
                    <el-tab-pane :label="t('settings.custom_command_search')" style="height: 100%">
                        <div style="display: flex; flex-direction: column; height: 100%;">
                            <el-button class="mt-4" style="width: 100%;  flex-shrink: 0;" @click="addCustomCommand">
                                {{ t('settings.add_item') }}
                            </el-button>
                            <el-table :data="custom_command" stripe
                                style="width: 100%;flex-grow: 1; height: 0; min-height: 0; margin-top: 10px;">
                                <el-table-column :label="t('settings.keyword_for_search')" show-overflow-tooltip
                                    fixed="left" width="150">
                                    <template #default="scope">
                                        <el-input v-model="custom_command[scope.$index][0]" size="small"
                                            :placeholder="t('settings.enter_keyword')"
                                            @change="updateCustomCommand"></el-input>
                                    </template>
                                </el-table-column>
                                <el-table-column :label="t('settings.command_content')" show-overflow-tooltip>
                                    <template #default="scope">
                                        <el-input v-model="custom_command[scope.$index][1]" size="small"
                                            :placeholder="t('settings.enter_command_content')"
                                            @change="updateCustomCommand"></el-input>
                                    </template>
                                </el-table-column>
                                <el-table-column fixed="right" :label="t('settings.actions')" width="100">
                                    <template #default="scope">
                                        <el-button link size="small" type="danger"
                                            @click="deleteCustomCommand(scope.$index)">
                                            {{ t('settings.delete_row') }}
                                        </el-button>
                                    </template>
                                </el-table-column>
                            </el-table>
                        </div>
                    </el-tab-pane>

                </el-tabs>
            </section>

            <section v-if="activeIndex === 4" class="page">
                <div style="display: flex; flex-direction: column; height: 100%;">
                    <el-button class="mt-4" style="width: 100%; flex-shrink: 0;" @click="refreshProgramInfo">
                        {{ t('settings.click_refresh') }}
                    </el-button>
                    <el-table-v2 :columns="columns" :data="programInfoList" :width="1000" :height="600" fixed
                        style="width: 100%;flex-grow: 1; margin-top: 10px;" />
                </div>
            </section>

            <section v-if="activeIndex === 5" class="page">
                <ConfigPathSelector></ConfigPathSelector>
            </section>

            <section v-if="activeIndex === 6" class="page">
                <ShortcutSetting></ShortcutSetting>
            </section>

            <section v-if="activeIndex === 7" class="page">
                <about></about>
            </section>

            <section v-if="activeIndex === 999" class="page">
                <debug></debug>
            </section>
        </div>
    </div>

    <el-dialog v-if="editingProgram" v-model="dialogVisible"
        :title="t('settings.edit_program_alias', { name: editingProgram.name })" width="500">
        <div style="display: flex; flex-direction: column; gap: 10px;">
            <div v-for="(alias, index) in program_alias[editingProgram.path]" :key="index"
                style="display: flex; align-items: center; gap: 10px;">
                <el-input :model-value="alias" @update:modelValue="newValue => updateAliasInDialog(index, newValue)"
                    :placeholder="t('settings.enter_alias')" />
                <el-button type="danger" @click="removeAliasInDialog(index)">{{ t('settings.delete') }}</el-button>
            </div>
        </div>
        <template #footer>
            <div class="dialog-footer">
                <el-button @click="addAliasInDialog" style="width: 100%; margin-bottom: 10px;">{{
                    t('settings.add_alias') }}</el-button>
                <el-button type="primary" @click="dialogVisible = false">{{ t('settings.close') }}</el-button>
            </div>
        </template>
    </el-dialog>
</template>

<script lang="ts" setup>
import { useI18n } from 'vue-i18n';
import { ref, onMounted, computed, onUnmounted, h } from 'vue';
import {
    Setting,
    Brush,
    Search,
    Connection,
    InfoFilled,
    List,
    Monitor
} from '@element-plus/icons-vue';

const { t } = useI18n();

import { invoke } from '@tauri-apps/api/core';
import { initializeLanguage } from '../i18n/index';
import { ElButton, ElInput, ElMessage, ElTag } from 'element-plus';
import ProgramIndex from './ProgramIndex.vue';
import { useRemoteConfigStore } from '../stores/remote_config';
import { storeToRefs } from 'pinia';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import UIConfigSetting from './UIConfigSetting.vue'
import about from "./about.vue";
import debug from "./debug.vue";
import ConfigPathSelector from "./ConfigPathSelector.vue";
import ShortcutSetting from './ShortcutSetting.vue';
import AppConfigSetting from './AppConfigSetting.vue';
const configStore = useRemoteConfigStore()
const { config } = storeToRefs(configStore)
interface MenuItem {
    title: string;
    icon: any;
}



const activeIndex = ref(0);

const menuItems = computed<MenuItem[]>(() => [
    { title: t('settings.menu.general'), icon: Setting },
    { title: t('settings.menu.appearance'), icon: Brush },
    { title: t('settings.menu.program_search'), icon: Search },
    { title: t('settings.menu.other_search'), icon: Search },
    { title: t('settings.menu.all_programs'), icon: List },
    { title: t('settings.menu.remote_management'), icon: Connection },
    { title: t('settings.menu.shortcuts'), icon: Search },
    { title: t('settings.menu.about'), icon: InfoFilled }
]);


const index_web_pages = computed({
    get: () => config.value.program_manager_config.loader.index_web_pages,
    set: (value) => {
        configStore.updateConfig({
            program_manager_config: {
                loader: { index_web_pages: value }
            }
        })
    }
})

const deleteIndexWebPages = (index: number) => {
    index_web_pages.value = index_web_pages.value.filter((_, i) => i !== index)
}

const updateIndexWebPages = () => {
    configStore.updateConfig({
        program_manager_config: {
            loader: { index_web_pages: index_web_pages.value }
        }
    })
}

const addIndexWebPage = () => {
    index_web_pages.value = [...index_web_pages.value, ["", ""]]
}

const custom_command = computed({
    get: () => config.value.program_manager_config.loader.custom_command,
    set: (value) => {
        configStore.updateConfig({
            program_manager_config: {
                loader: { custom_command: value }
            }
        })
    }
})

const deleteCustomCommand = (index: number) => {
    custom_command.value = custom_command.value.filter((_, i) => i !== index)
}

const updateCustomCommand = () => {
    configStore.updateConfig({
        program_manager_config: {
            loader: { custom_command: custom_command.value }
        }
    })
}

const addCustomCommand = () => {
    custom_command.value = [...custom_command.value, ["", ""]]
}

// 程序别名管理
const program_alias = computed({
    get: () => config.value.program_manager_config.loader.program_alias,
    set: (value) => {
        console.log(t('settings.update_pinia'));
        configStore.updateConfig({
            program_manager_config: {
                loader: { program_alias: value }
            }
        })
    }
})
const columns = computed(() => [
    { key: 'name', dataKey: 'name', title: t('settings.program_name'), width: 150 },
    { key: 'is_uwp', dataKey: 'is_uwp', title: t('settings.is_uwp_program'), width: 120 },
    { key: 'bias', dataKey: 'bias', title: t('settings.fixed_offset'), width: 100 },
    { key: 'history_launch_time', dataKey: 'history_launch_time', title: t('settings.launch_count'), width: 100 },
    { key: 'path', dataKey: 'path', title: t('settings.path'), width: 200 },
    {
        key: 'aliases',
        title: t('settings.aliases'),
        width: 300,
        cellRenderer: ({ rowData }: { rowData: ProgramInfo }) => {
            const aliasList = program_alias.value[rowData.path] || [];

            // 使用 El-Tag 展示别名
            const tags = aliasList.map(alias =>
                h(ElTag, { style: 'margin-right: 5px; margin-bottom: 5px;', type: 'info', size: 'small' }, () => alias)
            );

            // 编辑按钮
            const editButton = h(ElButton, {
                size: 'small',
                type: 'primary',
                link: true, // 使用链接样式，更简洁
                onClick: () => handleEditAliases(rowData)
            }, () => t('settings.manage_aliases'));

            // 将标签和按钮包裹在一个 div 中
            return h('div', { style: 'display: flex; flex-wrap: wrap; align-items: center;' }, [...tags, editButton]);
        }
    }
]);

// 用于控制对话框的状态
const dialogVisible = ref(false)
const editingProgram = ref<ProgramInfo | null>(null)

// 打开对话框的方法
const handleEditAliases = (rowData: ProgramInfo) => {
    editingProgram.value = { ...rowData }; // 浅拷贝一份，避免直接修改表格数据
    dialogVisible.value = true;
}

// Dialog 内的别名操作方法 (基本逻辑不变，只是操作对象从 rowData 变为 editingProgram)
const addAliasInDialog = () => {
    if (!editingProgram.value) return;
    const path = editingProgram.value.path;
    const currentAliases = program_alias.value[path] || [];
    const newAliases = { ...program_alias.value };
    newAliases[path] = [...currentAliases, ""];
    program_alias.value = newAliases;
}

const updateAliasInDialog = (index: number, newValue: string) => {
    if (!editingProgram.value) return;
    const path = editingProgram.value.path;
    const newProgramAlias = { ...program_alias.value };
    const currentAliases = [...(newProgramAlias[path] || [])];
    if (index >= 0 && index < currentAliases.length) {
        currentAliases[index] = newValue;
    }
    newProgramAlias[path] = currentAliases;
    program_alias.value = newProgramAlias;
}

const removeAliasInDialog = (index: number) => {
    if (!editingProgram.value) return;
    const path = editingProgram.value.path;
    const currentAliases = program_alias.value[path] || [];
    if (index >= 0 && index < currentAliases.length) {
        const newAliases = { ...program_alias.value };
        newAliases[path] = currentAliases.filter((_, i) => i !== index);
        if (newAliases[path].length === 0) {
            delete newAliases[path];
        }
        program_alias.value = newAliases;
    }
}

interface ProgramInfo {
    name: string
    is_uwp: boolean
    bias: number
    history_launch_time: number
    path: string
}

const programInfoList = ref<ProgramInfo[]>([])

const refreshProgramInfo = async () => {
    try {
        const data = await invoke<ProgramInfo[]>('get_program_info')
        programInfoList.value = data
    } catch (error) {
        console.error(t('settings.get_program_info_failed'), error)
    }
}

const save_config = async () => {
    await configStore.syncConfig()
    ElMessage({
        message: t('settings.config_saved'),
        type: 'success',
    })
}

let unlisten: Array<UnlistenFn | null> = [];

onMounted(async () => {
    await configStore.loadConfig()
    // 应用语言设置
    initializeLanguage(config.value.app_config.language)
    unlisten.push(await listen('emit_update_setting_window_config', async () => {
        await configStore.loadConfig()
        // 配置更新后重新应用语言设置
        initializeLanguage(config.value.app_config.language)
    }))
})

onUnmounted(async () => {
    unlisten.forEach(unlistenFn => {
        if (unlistenFn) unlistenFn();
    });
    unlisten = [];
})

</script>

<style>
body {
    margin: 0;
    padding: 0;
    overflow: hidden;
}
</style>

<style scoped>
.settings-container {
    display: flex;
    width: 100%;
    height: 100vh;
    background-color: #fff;
    overflow: hidden;
}

.sidebar {
    width: 180px;
    background-color: #f5f7fa;
    display: flex;
    flex-direction: column;
    border-right: 1px solid #e6e6e6;
    position: sticky;
    top: 0;
    height: 100vh;
    flex-shrink: 0;
    overflow-y: auto;
    overflow: hidden;
}

.header {
    padding: 16px;
    display: flex;
    align-items: center;
    border-bottom: 1px solid #e6e6e6;
    flex-shrink: 0;
}

.logo {
    width: 24px;
    height: 24px;
    margin-right: 8px;
}

.title {
    font-size: 16px;
    font-weight: 500;
}

.menu-item {
    display: flex;
    align-items: center;
    padding: 16px;
    cursor: pointer;
    transition: background-color 0.3s;
}

.menu-container {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
}

.menu-item:hover {
    background-color: #e9ecf2;
}

.menu-item.active {
    background-color: #e9ecf2;
}

.menu-text {
    margin-left: 12px;
    font-size: 14px;
}

.footer-item {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
    margin-top: auto;
    border-top: 1px solid #e6e6e6;
    flex-shrink: 0;
}

.custom-action-button:hover {
    background-color: #66b1ff;
    box-shadow: 0 4px 12px rgba(64, 158, 255, 0.4);
    transform: translateY(-1px);
}

.custom-action-button:active {
    background-color: #3a8ee6;
    box-shadow: 0 2px 4px rgba(64, 158, 255, 0.3);
    transform: translateY(0);
}

.footer-icon {
    margin-right: 8px;
    font-size: 16px;
}

.content {
    box-sizing: border-box;
    flex: 1;
    padding: 20px;
    height: 100vh;
    min-width: 0;
}

.el-question-icon {
    margin-left: 8px;
}


.el-icon {
    font-size: 18px;
    color: #606266;
}

.page {
    height: 100%;
    min-width: 0;
}
</style>
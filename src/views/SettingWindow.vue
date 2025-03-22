<template>
    <div class="settings-container">
        <div class="sidebar">
            <div class="header">
                <img src="../assets/icon.svg" alt="Logo" class="logo">
                <span class="title">选项</span>
            </div>

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
                <span class="menu-text">调试模式</span>
            </div>

            <div class="footer-item">
                <el-button type="primary" @click="save_config" :disabled="activeIndex >= 4">
                    <span>保存配置文件</span>
                </el-button>
            </div>
        </div>

        <!-- 内容区域 -->
        <div class="content">
            <!-- 常规设置 -->
            <section v-if="activeIndex === 0" class="page">
                <el-form label-width="auto" style="max-width: 600px">

                    <el-form-item label="自定义搜索栏的提示文本">
                        <el-input v-model="config.app_config.search_bar_placeholder" placeholder="Hello, ZeroLaunch!"
                            @change="(val: string) => configStore.updateConfig({ app_config: { search_bar_placeholder: val } })" />
                    </el-form-item>

                    <el-form-item label="自定义底部提示栏">
                        <el-input v-model="config.app_config.tips" placeholder="ZeroLaunch-rs v0.4.0"
                            @change="(val: string) => configStore.updateConfig({ app_config: { tips: val } })" />
                    </el-form-item>

                    <el-form-item label="设置开机自启动">
                        <el-switch v-model="config.app_config.is_auto_start"
                            @change="(val: boolean) => configStore.updateConfig({ app_config: { is_auto_start: val } })" />
                    </el-form-item>

                    <el-form-item label="设置静默启动">
                        <el-switch v-model="config.app_config.is_silent_start"
                            @change="(val: boolean) => configStore.updateConfig({ app_config: { is_silent_start: val } })" />
                    </el-form-item>

                    <el-form-item label="设置搜索结果数量">
                        <el-input-number v-model="config.app_config.search_result_count" :step="1" :precision="0"
                            :min="1"
                            @change="(val: number) => configStore.updateConfig({ app_config: { search_result_count: val } })" />
                    </el-form-item>

                    <el-form-item label="自动刷新数据库的时间">
                        <el-input-number v-model="config.app_config.auto_refresh_time" :step="1" :precision="0" :min="1"
                            @change="(val: number) => configStore.updateConfig({ app_config: { auto_refresh_time: val } })">
                            <template #suffix>
                                <span>分钟</span>
                            </template>
                        </el-input-number>
                    </el-form-item>

                    <el-form-item label="当唤醒程序失败时启动新实例">
                        <el-switch v-model="config.app_config.launch_new_on_failure"
                            @change="(val: boolean) => configStore.updateConfig({ app_config: { launch_new_on_failure: val } })" />
                    </el-form-item>

                    <el-form-item label="esc键优先关闭窗口">
                        <el-switch v-model="config.app_config.is_esc_hide_window_priority"
                            @change="(val: boolean) => configStore.updateConfig({ app_config: { is_esc_hide_window_priority: val } })" />
                        <el-tooltip class="box-item" effect="dark" placement="right-start" content="默认优先清空搜索栏后关闭">
                            <el-icon class="el-question-icon">
                                <QuestionFilled />
                            </el-icon>
                        </el-tooltip>
                    </el-form-item>

                    <el-form-item label="启用拖动窗口">
                        <el-switch v-model="config.app_config.is_enable_drag_window"
                            @change="(val: boolean) => configStore.updateConfig({ app_config: { is_enable_drag_window: val } })" />
                        <el-tooltip class="box-item" effect="dark" placement="right-start"
                            content="程序在下一次打开时会记忆上次的关闭的位置">
                            <el-icon class="el-question-icon">
                                <QuestionFilled />
                            </el-icon>
                        </el-tooltip>
                    </el-form-item>

                    <el-form-item label="调试模式">
                        <el-switch v-model="config.app_config.is_debug_mode"
                            @change="(val: boolean) => configStore.updateConfig({ app_config: { is_debug_mode: val } })" />
                    </el-form-item>
                </el-form>
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
                    <el-tab-pane label="自定义网址搜索">
                        <el-table :data="index_web_pages" stripe style="width: 100%; height: 100%">
                            <el-table-column label="关键字（用于搜索程序的匹配）" show-overflow-tooltip fixed="left" width="150">
                                <template #default="scope">
                                    <el-input v-model="index_web_pages[scope.$index][0]" size="small"
                                        placeholder="请输入关键字" @change="updateIndexWebPages"></el-input>
                                </template>
                            </el-table-column>
                            <el-table-column label="目标网站的地址" show-overflow-tooltip>
                                <template #default="scope">
                                    <el-input v-model="index_web_pages[scope.$index][1]" size="small"
                                        placeholder="请输入目标路径" @change="updateIndexWebPages"></el-input>
                                </template>
                            </el-table-column>
                            <el-table-column fixed="right" label="操作" width="100">
                                <template #default="scope">
                                    <el-button link size="small" type="danger"
                                        @click="deleteIndexWebPages(scope.$index)">
                                        删除一行
                                    </el-button>
                                </template>
                            </el-table-column>
                        </el-table>
                        <el-button class="mt-4" style="width: 100%" @click="addIndexWebPage">
                            添加项目
                        </el-button>
                    </el-tab-pane>
                    <el-tab-pane label="自定义命令搜索">
                        <el-table :data="custom_command" stripe style="width: 100%; height: 100%">
                            <el-table-column label="关键字（用于搜索程序的匹配）" show-overflow-tooltip fixed="left" width="150">
                                <template #default="scope">
                                    <el-input v-model="custom_command[scope.$index][0]" size="small"
                                        placeholder="请输入关键字" @change="updateCustomCommand"></el-input>
                                </template>
                            </el-table-column>
                            <el-table-column label="命令内容" show-overflow-tooltip>
                                <template #default="scope">
                                    <el-input v-model="custom_command[scope.$index][1]" size="small"
                                        placeholder="请输入命令内容" @change="updateCustomCommand"></el-input>
                                </template>
                            </el-table-column>
                            <el-table-column fixed="right" label="操作" width="100">
                                <template #default="scope">
                                    <el-button link size="small" type="danger"
                                        @click="deleteCustomCommand(scope.$index)">
                                        删除一行
                                    </el-button>
                                </template>
                            </el-table-column>
                        </el-table>
                        <el-button class="mt-4" style="width: 100%" @click="addCustomCommand">
                            添加项目
                        </el-button>
                    </el-tab-pane>

                </el-tabs>
            </section>

            <section v-if="activeIndex === 4" class="page">
                <ConfigPathSelector></ConfigPathSelector>
            </section>

            <section v-if="activeIndex === 5" class="page">
                <el-button class="mt-4" style="width: 100%" @click="refreshProgramInfo">
                    点击刷新
                </el-button>
                <el-table :data="programInfoList" stripe style="width: 100%; height: 100%">
                    <el-table-column label="程序名" prop="name" sortable show-overflow-tooltip>
                        <template #default="{ row }">
                            {{ row.name }}
                        </template>
                    </el-table-column>
                    <el-table-column label="是否是UWP程序" prop="is_uwp" sortable show-overflow-tooltip width="100">
                        <template #default="{ row }">
                            {{ row.is_uwp }}
                        </template>
                    </el-table-column>
                    <el-table-column label="固定偏移量" prop="bias" sortable show-overflow-tooltip width="100">
                        <template #default="{ row }">
                            {{ row.bias }}
                        </template>
                    </el-table-column>
                    <el-table-column label="启动次数" prop="history_launch_time" sortable show-overflow-tooltip width="100">
                        <template #default="{ row }">
                            {{ row.history_launch_time }}
                        </template>
                    </el-table-column>
                    <el-table-column label="路径" prop="path" sortable show-overflow-tooltip>
                        <template #default="{ row }">
                            {{ row.path }}
                        </template>
                    </el-table-column>
                </el-table>
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
</template>

<script lang="ts" setup>
import { ref, onMounted, computed, onUnmounted } from 'vue';
import {
    Setting,
    Brush,
    Search,
    Connection,
    InfoFilled,
    List,
    Monitor, QuestionFilled
} from '@element-plus/icons-vue';

import { invoke } from '@tauri-apps/api/core';
import { ElMessage } from 'element-plus';
import ProgramIndex from './ProgramIndex.vue';
import { open } from '@tauri-apps/plugin-dialog';
import { useRemoteConfigStore } from '../stores/remote_config';
import { storeToRefs } from 'pinia';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import UIConfigSetting from './UIConfigSetting.vue'
import about from "./about.vue";
import debug from "./debug.vue";
import ConfigPathSelector from "./ConfigPathSelector.vue";
import ShortcutSetting from './ShortcutSetting.vue';
import { DirectoryConfig } from '../api/remote_config_types';
const configStore = useRemoteConfigStore()
const { config } = storeToRefs(configStore)
interface MenuItem {
    title: string;
    icon: any;
}



const activeIndex = ref(0);

const menuItems: MenuItem[] = [
    { title: '常规设置', icon: Setting },
    { title: '外观设置', icon: Brush },
    { title: '程序搜索', icon: Search },
    { title: '其他搜索', icon: Search },
    { title: '远程管理', icon: Connection },
    { title: '所有程序', icon: List },
    { title: '快捷键设置', icon: Search },
    { title: '关于', icon: InfoFilled }
];


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
        console.error('获取程序信息失败:', error)
    }
}

const save_config = async () => {
    await configStore.syncConfig()
    ElMessage({
        message: '配置文件已保存',
        type: 'success',
    })
}

let unlisten: Array<UnlistenFn | null> = [];

onMounted(async () => {
    await configStore.loadConfig()
    unlisten.push(await listen('emit_update_setting_window_config', async () => {
        console.log("收到")
        await configStore.loadConfig()
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
    overflow-y: auto;
}

.header {
    padding: 16px;
    display: flex;
    align-items: center;
    border-bottom: 1px solid #e6e6e6;
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
    flex: 1;
    padding: 20px;
    overflow-y: auto;
    height: 100vh;
}

.el-question-icon {
    margin-left: 8px;
}


.el-icon {
    font-size: 18px;
    color: #606266;
}

.page {
    height: 90%;

}
</style>
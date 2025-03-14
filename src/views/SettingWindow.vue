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
                            @update:model-value="(val: string) => configStore.updateConfig({ app_config: { search_bar_placeholder: val } })" />
                    </el-form-item>

                    <el-form-item label="自定义底部提示栏">
                        <el-input v-model="config.app_config.tips" placeholder="ZeroLaunch-rs v0.4.0"
                            @update:model-value="(val: string) => configStore.updateConfig({ app_config: { tips: val } })" />
                    </el-form-item>

                    <el-form-item label="设置开机自启动">
                        <el-switch v-model="config.app_config.is_auto_start"
                            @update:model-value="(val: boolean) => configStore.updateConfig({ app_config: { is_auto_start: val } })" />
                    </el-form-item>

                    <el-form-item label="设置静默启动">
                        <el-switch v-model="config.app_config.is_silent_start"
                            @update:model-value="(val: boolean) => configStore.updateConfig({ app_config: { is_silent_start: val } })" />
                    </el-form-item>

                    <el-form-item label="设置搜索结果数量">
                        <el-input-number v-model="config.app_config.search_result_count" :step="1" :precision="0"
                            @update:model-value="(val: number) => configStore.updateConfig({ app_config: { search_result_count: val } })" />
                    </el-form-item>

                    <el-form-item label="自动刷新数据库的时间（分钟）">
                        <el-input-number v-model="config.app_config.auto_refresh_time" :step="1" :precision="0"
                            @update:model-value="(val: number) => configStore.updateConfig({ app_config: { auto_refresh_time: val } })" />
                    </el-form-item>

                    <el-form-item label="当唤醒程序失败时启动新实例">
                        <el-switch v-model="config.app_config.launch_new_on_failure"
                            @update:model-value="(val: boolean) => configStore.updateConfig({ app_config: { launch_new_on_failure: val } })" />
                    </el-form-item>

                    <el-form-item label="调试模式">
                        <el-switch v-model="config.app_config.is_debug_mode"
                            @update:model-value="(val: boolean) => configStore.updateConfig({ app_config: { is_debug_mode: val } })" />
                    </el-form-item>
                </el-form>

            </section>

            <section v-if="activeIndex === 1" class="page">

                <el-form-item label="搜索栏与状态栏的背景颜色">
                    <el-color-picker v-model="config.ui_config.search_bar_background_color" show-alpha
                        @update:model-value="(val: string) => configStore.updateConfig({ ui_config: { search_bar_background_color: rgbaToHex(val) } })" />
                </el-form-item>
                <el-form-item label="设置结果栏的背景颜色">
                    <el-color-picker v-model="config.ui_config.selected_item_color" show-alpha
                        @update:model-value="(val: string) => configStore.updateConfig({ ui_config: { selected_item_color: rgbaToHex(val) } })" />
                </el-form-item>
                <el-form-item label="搜索栏字体的颜色">
                    <el-color-picker v-model="config.ui_config.search_bar_font_color"
                        @update:model-value="(val: string) => configStore.updateConfig({ ui_config: { search_bar_font_color: rgbaToHex(val) } })" />
                </el-form-item>
                <el-form-item label="设置结果栏的字体颜色">
                    <el-color-picker v-model="config.ui_config.item_font_color"
                        @update:model-value="(val: string) => configStore.updateConfig({ ui_config: { item_font_color: rgbaToHex(val) } })" />
                </el-form-item>
                <el-form-item label="搜索栏的字体大小(单位rem)">
                    <el-input-number v-model="config.ui_config.search_bar_font_size" placeholder="2" :min="0"
                        :step="0.1"
                        @update:model-value="(val: number) => configStore.updateConfig({ ui_config: { search_bar_font_size: val } })" />
                </el-form-item>
                <el-form-item label="结果栏的字体大小(单位rem)">
                    <el-input-number v-model="config.ui_config.item_font_size" placeholder="1.3" :min="0" :step="0.1"
                        @update:model-value="(val: number) => configStore.updateConfig({ ui_config: { item_font_size: val } })" />
                </el-form-item>
                <el-form-item label="窗口垂直方向偏移比例因子">
                    <el-tooltip class="box-item" effect="dark" content="0表示在屏幕顶部，1表示在屏幕底部，0.5表示在屏幕正中间"
                        placement="right-start">
                        <el-input-number v-model="config.ui_config.vertical_position_ratio" placeholder="0.4" :min="0"
                            :step="0.05" :max="1"
                            @update:model-value="(val: number) => configStore.updateConfig({ ui_config: { vertical_position_ratio: val } })" />
                    </el-tooltip>
                </el-form-item>

                <el-form-item label="搜索栏的高度(单位px)">
                    <el-input-number v-model="config.ui_config.search_bar_height" placeholder="65" :min="1" :step="1"
                        :precision="0"
                        @update:model-value="(val: number) => configStore.updateConfig({ ui_config: { search_bar_height: val } })" />
                </el-form-item>

                <el-form-item label="结果栏中一项的高度(单位px)">
                    <el-input-number v-model="config.ui_config.result_item_height" placeholder="62" :min="1" :step="1"
                        :precision="0"
                        @update:model-value="(val: number) => configStore.updateConfig({ ui_config: { result_item_height: val } })" />
                </el-form-item>

                <el-form-item label="底栏的高度(单位px)">
                    <el-input-number v-model="config.ui_config.footer_height" placeholder="42" :min="0" :step="1"
                        :precision="0"
                        @update:model-value="(val: number) => configStore.updateConfig({ ui_config: { footer_height: val } })" />
                </el-form-item>

                <el-form-item label="程序的宽度(单位px)">
                    <el-input-number v-model="config.ui_config.window_width" placeholder="1000" :min="400" :step="1"
                        :precision="0"
                        @update:model-value="(val: number) => configStore.updateConfig({ ui_config: { window_width: val } })" />
                </el-form-item>

                <el-form-item label="选择背景图片">
                    <el-button type="primary" @click="select_background_picture">选择图片</el-button>
                    <el-button type="danger" @click="delete_background_picture">删除图片</el-button>

                </el-form-item>
                <el-form-item label="计算一个图片的主题色">
                    <el-button type="primary" @click="get_dominant_color">选择图片</el-button>
                    <div v-if="dominant_color"> 该图片的主题色为: {{ dominant_color }} </div>
                </el-form-item>
            </section>


            <!-- 外观设置 -->
            <section v-if="activeIndex === 2" class="page">
                <el-tabs style="height: 100% " class="demo-tabs">
                    <el-tab-pane label="设置遍历路径">
                        <el-table :data="targetPath" stripe style="width: 100%; height: 100%">
                            <el-table-column label="目标路径" show-overflow-tooltip>
                                <template #default="{ row, $index }">
                                    <el-input v-model="row[0]" size="small" placeholder="请输入目标路径"
                                        @change="updateTargetPath($index, row)"></el-input>
                                </template>
                            </el-table-column>
                            <el-table-column label="遍历深度" show-overflow-tooltip fixed="right" width="80">
                                <template #default="{ row, $index }">
                                    <el-input-number v-model="row[1]" size="small" placeholder="请输入遍历深度" :precision="0"
                                        :min="1" @change="updateTargetPath($index, row)"></el-input-number>
                                </template>
                            </el-table-column>
                            <el-table-column fixed="right" label="操作" width="100">
                                <template #default="scope">
                                    <el-button link size="small" type="danger"
                                        @click="deleteTargetPathRow(scope.$index)">
                                        删除一行
                                    </el-button>
                                </template>
                            </el-table-column>
                        </el-table>

                        <el-button class="mt-4" style="width: 100%" @click="addTargetPath">
                            添加项目
                        </el-button>
                    </el-tab-pane>

                    <el-tab-pane label="设置屏蔽路径">
                        <el-table :data="forbidden_paths" stripe style="width: 100%; height: 100%">
                            <el-table-column label="目标屏蔽路径" show-overflow-tooltip>
                                <template #default="{ $index }">
                                    <el-input v-model="forbidden_paths[$index]" size="small" placeholder="请输入目标路径"
                                        @change="updateForbiddenPaths"></el-input>
                                </template>
                            </el-table-column>
                            <el-table-column fixed="right" label="操作" width="100">
                                <template #default="{ $index }">
                                    <el-button link size="small" type="danger" @click="deleteForbiddenPath($index)">
                                        删除一行
                                    </el-button>
                                </template>
                            </el-table-column>
                        </el-table>
                        <el-button class="mt-4" style="width: 100%" @click="addForbiddenPath">
                            添加项目
                        </el-button>
                    </el-tab-pane>

                    <el-tab-pane label="设置屏蔽关键字">
                        <el-table :data="forbidden_program_key" stripe style="width: 100%; height: 100%">
                            <el-table-column label="目标关键字" show-overflow-tooltip>
                                <template #default="{ $index }">
                                    <el-input v-model="forbidden_program_key[$index]" size="small"
                                        placeholder="请输入目标关键字" @change="updateForbiddenProgramKey"></el-input>
                                </template>
                            </el-table-column>
                            <el-table-column fixed="right" label="操作" width="100">
                                <template #default="{ $index }">
                                    <el-button link size="small" type="danger" @click="deleteForbiddenKey($index)">
                                        删除一行
                                    </el-button>
                                </template>
                            </el-table-column>
                        </el-table>
                        <el-button class="mt-4" style="width: 100%" @click="addForbiddenKey">
                            添加项目
                        </el-button>
                    </el-tab-pane>


                    <el-tab-pane label="设置固定偏移量">
                        <el-table :data="keyFilterData" stripe style="width: 100%; height: 100%">
                            <el-table-column label="目标关键字">
                                <template #default="{ row }">
                                    <el-input v-model="row.key" size="small" placeholder="请输入目标关键字"
                                        @change="updateProgramBias(row)"></el-input>
                                </template>
                            </el-table-column>
                            <el-table-column label="偏移量" show-overflow-tooltip>
                                <template #default="{ row }">
                                    <el-input-number v-model="row.bias" size="small" placeholder="请输入偏移量"
                                        @change="updateProgramBias(row)"></el-input-number>
                                </template>
                            </el-table-column>
                            <el-table-column label="备注" show-overflow-tooltip>
                                <template #default="{ row }">
                                    <el-input v-model="row.note" size="small" placeholder="请输入备注"
                                        @change="updateProgramBias(row)"></el-input>
                                </template>
                            </el-table-column>
                            <el-table-column fixed="right" label="操作" width="100">
                                <template #default="{ $index }">
                                    <el-button link size="small" type="danger" @click="deleteKeyFilterRow($index)">
                                        删除一行
                                    </el-button>
                                </template>
                            </el-table-column>
                        </el-table>
                        <el-button class="mt-4" style="width: 100%" @click="addKeyFilter">
                            添加项目
                        </el-button>
                    </el-tab-pane>
                    <el-tab-pane label="额外设置">
                        <el-form-item label="扫描UWP应用">
                            <el-switch v-model="is_scan_uwp_programs" @change="updateIsScanUwpPrograms" />
                        </el-form-item>
                    </el-tab-pane>
                </el-tabs>
            </section>

            <section v-if="activeIndex === 3" class="page">
                <el-tabs style="height: 100%" class="demo-tabs">
                    <el-tab-pane label="自定义文件搜索">
                        <div class="mb-4">
                            <el-button type="primary" plain @click="handleSelectFile">选择并添加一个文件</el-button>
                        </div>

                        <el-table :data="index_file_paths" stripe style="width: 100%; height: 100%">
                            <el-table-column label="目标路径" show-overflow-tooltip>
                                <template #default="scope">
                                    <el-input v-model="index_file_paths[scope.$index]" size="small"
                                        placeholder="请输入目标路径" @change="updateIndexFilePaths"></el-input>
                                </template>
                            </el-table-column>
                            <el-table-column fixed="right" label="操作" width="100">
                                <template #default="scope">
                                    <el-button link size="small" type="danger"
                                        @click="deleteIndexFileRow(scope.$index)">
                                        删除一行
                                    </el-button>
                                </template>
                            </el-table-column>
                        </el-table>
                        <el-button class="mt-4" style="width: 100%" @click="addIndexFile">
                            添加项目
                        </el-button>
                    </el-tab-pane>

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
                    <el-table-column label="程序名" show-overflow-tooltip>
                        <template #default="{ row }">
                            {{ row.name }}
                        </template>
                    </el-table-column>
                    <el-table-column label="是否是UWP程序" show-overflow-tooltip width="100">
                        <template #default="{ row }">
                            {{ row.is_uwp }}
                        </template>
                    </el-table-column>
                    <el-table-column label="固定偏移量" show-overflow-tooltip width="100">
                        <template #default="{ row }">
                            {{ row.bias }}
                        </template>
                    </el-table-column>
                    <el-table-column label="启动次数" show-overflow-tooltip width="100">
                        <template #default="{ row }">
                            {{ row.history_launch_time }}
                        </template>
                    </el-table-column>
                    <el-table-column label="路径" show-overflow-tooltip>
                        <template #default="{ row }">
                            {{ row.path }}
                        </template>
                    </el-table-column>
                </el-table>
            </section>

            <section v-if="activeIndex === 6" class="page">
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
    Monitor
} from '@element-plus/icons-vue';

import { invoke } from '@tauri-apps/api/core';
import { ElMessage } from 'element-plus';
import { open } from '@tauri-apps/plugin-dialog';
import { useRemoteConfigStore } from '../stores/remote_config';
import { storeToRefs } from 'pinia';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { rgbaToHex } from '../utils/color';
import about from "./about.vue";
import debug from "./debug.vue";
import ConfigPathSelector from "./ConfigPathSelector.vue";

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
    { title: '关于', icon: InfoFilled }
];

const select_picture = async () => {
    const file_path = await open({
        canCreateDirectories: false,  // 禁止创建目录
        directory: false,             // 禁止选择目录
        multiple: false,              // 只允许选择一个文件
        title: "选择一个图片",         // 文件选择框的标题
        filters: [
            {
                name: 'Images',  // 过滤器的名称
                extensions: ['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp']  // 允许的图片文件扩展名
            }
        ]
    });
    return file_path;
}

const select_background_picture = async () => {
    let file_path = await select_picture();
    if (file_path) {
        console.log(file_path)
        invoke("select_background_picture", { path: file_path });
        ElMessage({
            message: '图片已保存',
            type: 'success',
        })
    }
}

const delete_background_picture = () => {
    invoke("select_background_picture", { path: "" });
    ElMessage({
        message: '图片已删除',
        type: 'success',
    })
}

const targetPath = computed({
    get: () => config.value.program_manager_config.loader.target_paths,
    set: (value) => {
        console.log("调用set方法")
        configStore.updateConfig({
            program_manager_config: {
                loader: { target_paths: value }
            }
        })
    }
})

const updateTargetPath = (index: number, value: [string, string]) => {
    console.log(value);
    const newTargetPaths = [...config.value.program_manager_config.loader.target_paths] as [string, number][]
    newTargetPaths[index] = [value[0], parseInt(value[1])]
    console.log([value[0], parseInt(value[1])])
    configStore.updateConfig({
        program_manager_config: {
            loader: {
                target_paths: newTargetPaths
            }
        }
    })
}

const deleteTargetPathRow = (index: number) => {
    const newTargetPaths = config.value.program_manager_config.loader.target_paths.filter((_, i) => i !== index)
    configStore.updateConfig({
        program_manager_config: {
            loader: {
                target_paths: newTargetPaths
            }
        }
    })
}

const addTargetPath = () => {
    const newTargetPaths = [...config.value.program_manager_config.loader.target_paths, ["", 1]] as [string, number][]
    configStore.updateConfig({
        program_manager_config: {
            loader: {
                target_paths: newTargetPaths
            }
        }
    })
}

const forbidden_paths = computed({
    get: () => config.value.program_manager_config.loader.forbidden_paths,
    set: (value) => {
        configStore.updateConfig({
            program_manager_config: {
                loader: { forbidden_paths: value }
            }
        })
    }
})

const updateForbiddenPaths = () => {
    configStore.updateConfig({
        program_manager_config: {
            loader: { forbidden_paths: forbidden_paths.value }
        }
    })
}

const deleteForbiddenPath = (index: number) => {
    const newPaths = [...forbidden_paths.value]
    newPaths.splice(index, 1)
    forbidden_paths.value = newPaths
}

const addForbiddenPath = () => {
    forbidden_paths.value = [...forbidden_paths.value, ""]
}

const forbidden_program_key = computed({
    get: () => config.value.program_manager_config.loader.forbidden_program_key,
    set: (value) => {
        configStore.updateConfig({
            program_manager_config: {
                loader: { forbidden_program_key: value }
            }
        })
    }
})

const updateForbiddenProgramKey = () => {
    configStore.updateConfig({
        program_manager_config: {
            loader: { forbidden_program_key: forbidden_program_key.value }
        }
    })
}

const deleteForbiddenKey = (index: number) => {
    const newKeys = [...forbidden_program_key.value]
    newKeys.splice(index, 1)
    forbidden_program_key.value = newKeys
}

const addForbiddenKey = () => {
    forbidden_program_key.value = [...forbidden_program_key.value, ""]
}

const is_scan_uwp_programs = computed({
    get: () => config.value.program_manager_config.loader.is_scan_uwp_programs,
    set: (value) => {
        configStore.updateConfig({
            program_manager_config: {
                loader: { is_scan_uwp_programs: value }
            }
        })
    }
})

const updateIsScanUwpPrograms = () => {
    configStore.updateConfig({
        program_manager_config: {
            loader: { is_scan_uwp_programs: is_scan_uwp_programs.value }
        }
    })
}

interface KeyFilterData {
    originalKey: string
    key: string
    bias: number
    note: string
}

const keyFilterData = computed(() => {
    const bias = config.value.program_manager_config.loader.program_bias;
    return Object.keys(bias).map(key => ({
        originalKey: key,  // 初始化时保存原始键
        key,
        bias: bias[key][0],
        note: bias[key][1] || ''
    }));
});

const updateProgramBias = (row: KeyFilterData) => {
    const newProgramBias = { ...config.value.program_manager_config.loader.program_bias }

    // 检查是否存在原始键（仅当数据结构包含originalKey时）
    if (row.originalKey !== row.key) {
        delete newProgramBias[row.originalKey] // 删除旧键
    }

    newProgramBias[row.key] = [row.bias, row.note] // 更新或新增键

    configStore.updateConfig({
        program_manager_config: {
            loader: {
                program_bias: newProgramBias
            }
        }
    })
}


const deleteKeyFilterRow = (index: number) => {
    // 深拷贝 program_bias
    const newProgramBias = JSON.parse(JSON.stringify(config.value.program_manager_config.loader.program_bias));
    const keyToDelete = keyFilterData.value[index].key;
    delete newProgramBias[keyToDelete];
    console.log("删除一行")

    console.log(newProgramBias)
    configStore.updateConfig({
        program_manager_config: {
            loader: {
                program_bias: newProgramBias
            }
        }
    })
}

const addKeyFilter = () => {
    const newProgramBias = { ...config.value.program_manager_config.loader.program_bias }
    const newKey = `请输入关键字`
    newProgramBias[newKey] = [0, '']

    configStore.updateConfig({
        program_manager_config: {
            loader: {
                program_bias: newProgramBias
            }
        }
    })
}

const handleSelectFile = async () => {
    const filePath = await open({ canCreateDirectories: false, directory: false, multiple: false, title: "选择一个文件" })
    if (filePath) {
        index_file_paths.value = [...index_file_paths.value, filePath as string]
    }
}

// 计算属性用于直接访问和修改 config.program_manager_config 中的数据
const index_file_paths = computed({
    get: () => config.value.program_manager_config.loader.index_file_paths,
    set: (value) => {
        configStore.updateConfig({
            program_manager_config: {
                loader: { index_file_paths: value }
            }
        })
    }
})

const updateIndexFilePaths = () => {
    configStore.updateConfig({
        program_manager_config: {
            loader: { index_file_paths: index_file_paths.value }
        }
    })
}

const deleteIndexFileRow = (index: number) => {
    index_file_paths.value = index_file_paths.value.filter((_, i) => i !== index)
}

const addIndexFile = () => {
    index_file_paths.value = [...index_file_paths.value, ""]
}

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
let dominant_color = ref<string | null>(null);


const get_dominant_color = async () => {
    let file_path = await select_picture();
    let ret = await invoke<string>('get_dominant_color', { path: file_path });
    dominant_color.value = ret;
}

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

.el-icon {
    font-size: 18px;
    color: #606266;
}
</style>
<template>
    <el-tabs v-model="activeName" class="demo-tabs" @tab-click="handleClick" drag>
        <el-tab-pane label="程序设置" name="a">
            <el-form :model="config" label-width="auto" style="max-width: 600px">

                <el-form-item label="自定义搜索栏的提示文本">
                    <el-input v-model="config.search_bar_placeholder" placeholder="Hello, ZeroLaunch!" />
                </el-form-item>

                <el-form-item label="自定义搜索无结果的文本">
                    <el-input v-model="config.search_bar_no_result" placeholder="当前搜索无结果" />
                </el-form-item>

                <el-form-item label="设置开机自启动">
                    <el-switch v-model="config.is_auto_start" />
                </el-form-item>

                <el-form-item label="设置静默启动">
                    <el-switch v-model="config.is_silent_start" />
                </el-form-item>

                <el-form-item label="设置搜索结果数量">
                    <el-input-number v-model="config.search_result_count" :step="1" :precision="0" />
                </el-form-item>

                <el-form-item label="自动刷新数据库的时间（分钟）">
                    <el-input-number v-model="config.auto_refresh_time" :step="1" :precision="0" />
                </el-form-item>

                <el-form-item label="设置选中项的背景颜色">
                    <el-color-picker v-model="config.selected_item_color" />
                </el-form-item>

                <el-form-item label="设置选中项的字体颜色">
                    <el-color-picker v-model="config.item_font_color" />
                </el-form-item>

                <el-form-item label="选择背景图片">
                    <el-button type="primary" @click="select_background_picture">选择图片</el-button>
                    <el-button type="danger" @click="delete_background_picture">删除图片</el-button>
                </el-form-item>
                <el-button type="primary" @click="save_app_config">提交</el-button>
            </el-form>
        </el-tab-pane>
        <el-tab-pane label="自定义搜索路径" name="b">
            <el-tabs tab-position="left" style="height: 100% " class="demo-tabs">
                <el-tab-pane label="设置遍历路径">
                    <el-table :data="path_data.target_paths" stripe style="width: 100%; height: 100%">
                        <el-table-column label="目标路径" show-overflow-tooltip>
                            <template #default="scope">
                                <el-input v-model="path_data.target_paths[scope.$index]" size="small"
                                    placeholder="请输入目标路径"></el-input>
                            </template>
                        </el-table-column>
                        <el-table-column fixed="right" label="操作" width="100">
                            <template #default="scope">
                                <el-button link size="small" type="danger" @click="deleteTargetPathRow(scope.$index)">
                                    删除一行
                                </el-button>
                            </template>
                        </el-table-column>
                    </el-table>

                    <el-button class="mt-4" style="width: 100%" @click="add_target_path">
                        Add Item
                    </el-button>
                </el-tab-pane>

                <el-tab-pane label="设置屏蔽路径">
                    <el-table :data="path_data.forbidden_paths" stripe style="width: 100%; height: 100%">
                        <el-table-column label="目标��径" show-overflow-tooltip>
                            <template #default="scope">
                                <el-input v-model="path_data.forbidden_paths[scope.$index]" size="small"
                                    placeholder="请输入目标路径"></el-input>
                            </template>
                        </el-table-column>
                        <el-table-column fixed="right" label="操作" width="100">
                            <template #default="scope">
                                <el-button link size="small" type="danger"
                                    @click="deleteForbiddenPathRow(scope.$index)">
                                    删除一行
                                </el-button>
                            </template>
                        </el-table-column>
                    </el-table>
                    <el-button class="mt-4" style="width: 100%" @click="add_forbidden_path">
                        Add Item
                    </el-button>

                </el-tab-pane>
                <el-tab-pane label="设置屏蔽关键字">
                    <el-table :data="path_data.forbidden_key" stripe style="width: 100%; height: 100%">
                        <el-table-column label="目标关键字" show-overflow-tooltip>
                            <template #default="scope">
                                <el-input v-model="path_data.forbidden_key[scope.$index]" size="small"
                                    placeholder="请输入目标关键字"></el-input>
                            </template>
                        </el-table-column>
                        <el-table-column fixed="right" label="操作" width="100">
                            <template #default="scope">
                                <el-button link size="small" type="danger" @click="deleteForbiddenKeyRow(scope.$index)">
                                    删除一行
                                </el-button>
                            </template>
                        </el-table-column>
                    </el-table>
                    <el-button class="mt-4" style="width: 100%" @click="add_forbidden_key">
                        Add Item
                    </el-button>
                </el-tab-pane>
                <el-tab-pane label="额外设置">
                    <el-form-item label="扫描UWP应用">
                        <el-switch v-model="path_data.is_scan_uwp_program" />
                    </el-form-item>

                </el-tab-pane>
            </el-tabs>
            <el-button type="primary" @click="save_path_config">提交</el-button>

        </el-tab-pane>
        <el-tab-pane label="关键字过滤设置" name="c">
            <el-table :data="key_data" stripe style="width: 100%; height: 100%">
                <el-table-column label="目标关键字" show-overflow-tooltip>
                    <template #default="scope">
                        <el-input v-model="key_data[scope.$index].key" size="small" placeholder="请输入目标关键字"></el-input>
                    </template>
                </el-table-column>
                <el-table-column label="偏移量" show-overflow-tooltip>
                    <template #default="scope">
                        <el-input-number v-model="key_data[scope.$index].bias" size="small"
                            placeholder="请输入偏移量"></el-input-number>
                    </template>
                </el-table-column>
                <el-table-column label="备注" show-overflow-tooltip>
                    <template #default="scope">
                        <el-input v-model="key_data[scope.$index].note" size="small" placeholder="请输入备注"></el-input>
                    </template>
                </el-table-column>
                <el-table-column fixed="right" label="操作" width="100">
                    <template #default="scope">
                        <el-button link size="small" type="danger" @click="deleteKeyFilterRow(scope.$index)">
                            删除一行
                        </el-button>
                    </template>
                </el-table-column>
            </el-table>
            <el-button class="mt-4" style="width: 100%" @click="add_key_filter">
                Add Item
            </el-button>
            <el-button type="primary" @click="save_key_filter_data">提交</el-button>
        </el-tab-pane>

        <el-tab-pane label="添加自定义索引文件" name="d" drag>
            <el-tabs tab-position="left" style="height: 100% " class="demo-tabs">
                <el-tab-pane label="索引文件">
                    <div class="mb-4">
                        <el-button type="primary" plain @click="handleSelectFile">选择一个文件</el-button>
                    </div>

                    <el-table :data="index_file_data" stripe style="width: 100%; height: 100%">
                        <el-table-column label="目标路径" show-overflow-tooltip>
                            <template #default="scope">
                                <el-input v-model="index_file_data[scope.$index]" size="small"
                                    placeholder="请输���目标路径"></el-input>
                            </template>
                        </el-table-column>
                        <el-table-column fixed="right" label="操作" width="100">
                            <template #default="scope">
                                <el-button link size="small" type="danger" @click="deleteIndexFileRow(scope.$index)">
                                    删除一行
                                </el-button>
                            </template>
                        </el-table-column>
                    </el-table>
                    <el-button class="mt-4" style="width: 100%" @click="add_index_file">
                        Add Item
                    </el-button>

                </el-tab-pane>
                <el-tab-pane label="索引网址">
                    <el-table :data="index_web_pages_data" stripe style="width: 100%; height: 100%">
                        <el-table-column label="关键字（用于搜索程序的匹配）" show-overflow-tooltip>
                            <template #default="scope">
                                <el-input v-model="index_web_pages_data[scope.$index].show_name" size="small"
                                    placeholder="请输入关键字"></el-input>
                            </template>
                        </el-table-column>
                        <el-table-column label="目标网站的地址" show-overflow-tooltip>
                            <template #default="scope">
                                <el-input v-model="index_web_pages_data[scope.$index].url" size="small"
                                    placeholder="请输入目标路径"></el-input>
                            </template>
                        </el-table-column>
                        <el-table-column fixed="right" label="操作" width="100">
                            <template #default="scope">
                                <el-button link size="small" type="danger" @click="deleteIndexWebPages(scope.$index)">
                                    删除一行
                                </el-button>
                            </template>
                        </el-table-column>
                    </el-table>
                    <el-button class="mt-4" style="width: 100%" @click="add_index_web_page">
                        Add Item
                    </el-button>
                </el-tab-pane>
            </el-tabs>
            <el-button type="primary" @click="save_custom_file_path">提交</el-button>
        </el-tab-pane>
        <el-tab-pane label="其它设置" name="e">
            <el-form-item label="设置配置文件的保存地址">
                <el-button type="primary" @click="change_remote_config_path_dir"> 选择目标路径</el-button>
                <el-input v-model="remote_config_path_dir" placeholder="设置配置文件保存路径" />
            </el-form-item>
        </el-tab-pane>
        <el-tab-pane label="查看当前索引的所有的程序" name="f">
            <el-button class="mt-4" style="width: 100%" @click="get_program_info">
                刷新列表
            </el-button>
            <el-table :data="program_info" stripe style="width: 100%; height: 100%">
                <el-table-column label="程序名" show-over flow-tooltip>
                    <template #default="scope">
                        {{ program_info[scope.$index].name }}
                    </template>
                </el-table-column>
                <el-table-column label="是否是UWP程序" show-overflow-tooltip width="100">
                    <template #default="scope">
                        {{ program_info[scope.$index].is_uwp }}
                    </template>
                </el-table-column>
                <el-table-column label="固定偏移量" show-overflow-tooltip width="100">
                    <template #default="scope">
                        {{ program_info[scope.$index].bias }}
                    </template>
                </el-table-column>
                <el-table-column label="启动次数" show-overflow-tooltip width="100">
                    <template #default="scope">
                        {{ program_info[scope.$index].history_launch_time }}
                    </template>
                </el-table-column>
                <el-table-column label="路径" show-overflow-tooltip>
                    <template #default="scope">
                        {{ program_info[scope.$index].path }}
                    </template>
                </el-table-column>
            </el-table>

        </el-tab-pane>
        <el-tab-pane label="关于" name="g" drag>
            项目地址： https://github.com/ghost-him/ZeroLaunch-rs
        </el-tab-pane>
    </el-tabs>
</template>
<script lang="ts" setup>
import { ref, reactive, onMounted, onUnmounted } from 'vue';
import type { TabsPaneContext } from 'element-plus';
import { invoke } from '@tauri-apps/api/core';
import { ElMessage } from 'element-plus';
import { open } from '@tauri-apps/plugin-dialog';

const activeName = ref('a')
const handleClick = (tab: TabsPaneContext, event: Event) => {
    console.log(tab, event)
}

const remote_config_path_dir = ref('');

const change_remote_config_path_dir = async () => {
    try {
        const selected = await open({
            canCreateDirectories: true, directory: true, multiple: false, title: "选择目标文件夹"
        });

        if (selected) {
            console.log('选择的文件夹路径:', selected);
            remote_config_path_dir.value = selected;
            // 调用后端
            await invoke('change_remote_config_dir', { configDir: selected });
            update_setting_window_info();
            // 在这里处理选中的文件夹路径
        } else {
            console.log('没有选择文件夹');
        }
    } catch (err) {
        console.error('选择文件夹时出错:', err);
    }
}

const handleSelectFile = async () => {
    const file_path = await open({ canCreateDirectories: false, directory: false, multiple: false, title: "选择一个文件" });
    if (file_path) {
        index_file_data.value.push(file_path)
    }
}

const select_background_picture = async () => {
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
    if (file_path) {
        console.log(file_path)
        invoke("select_background_picture", { path: file_path });
    }
    ElMessage({
        message: '图片已保存',
        type: 'success',
    })
}

const delete_background_picture = () => {
    invoke("select_background_picture", { path: "" });
    ElMessage({
        message: '图片已删除',
        type: 'success',
    })
}

interface app_config {
    search_bar_placeholder: string,
    search_bar_no_result: string,
    is_auto_start: boolean,
    is_silent_start: boolean,
    search_result_count: number,
    auto_refresh_time: number,
    selected_item_color: string,
    item_font_color: string,
}

// do not use same name with ref
const config = reactive<app_config>({
    search_bar_placeholder: '',
    search_bar_no_result: '',
    is_auto_start: false,
    is_silent_start: false,

    search_result_count: 4,
    auto_refresh_time: 30,
    selected_item_color: '',
    item_font_color: '',
})

interface PathData {
    target_paths: Array<string>;
    forbidden_paths: Array<string>;
    forbidden_key: Array<string>;
    is_scan_uwp_program: false;
}

const path_data = ref<PathData>({
    target_paths: [],
    forbidden_paths: [],
    forbidden_key: [],
    is_scan_uwp_program: false,
});

interface KeyFilterData {
    key: string;
    bias: number;
    note: string;
}

interface ProgramInfo {
    name: string;
    is_uwp: boolean;
    bias: number;
    path: string;
    history_launch_time: number;
}

const index_file_data = ref<Array<string>>([])

const get_index_file_data = async () => {
    const data = await invoke<Array<string>>('get_file_info');
    index_file_data.value = data
}

interface WebPages {
    show_name: string,
    url: string
}

const index_web_pages_data = ref<Array<WebPages>>([])

const get_index_web_pages = async () => {
    const data = await invoke<Array<WebPages>>('get_web_pages_infos');
    index_web_pages_data.value = data
}


const key_data = ref<Array<KeyFilterData>>([])
const program_info = ref<Array<ProgramInfo>>([])



const get_app_config = async () => {
    const loadedConfig = await invoke<app_config>('get_config')
    Object.assign(config, loadedConfig)
}

const save_custom_file_path = async () => {
    await invoke('save_custom_file_path', { webPages: index_web_pages_data.value, filePaths: index_file_data.value });
    ElMessage({
        message: '配置文件已保存',
        type: 'success',
    })
}

const save_app_config = async () => {
    await invoke('save_app_config', { appConfig: config })
    ElMessage({
        message: '配置文件已保存',
        type: 'success',
    })
}

const get_path_config = async () => {
    const data = await invoke<PathData>('get_path_config');

    path_data.value.target_paths = data.target_paths;
    path_data.value.forbidden_key = data.forbidden_key;
    path_data.value.forbidden_paths = data.forbidden_paths;
    path_data.value.is_scan_uwp_program = data.is_scan_uwp_program;
}

const get_program_info = async () => {
    const data = await invoke<Array<ProgramInfo>>('get_program_info');
    program_info.value = data;
    console.log(program_info)
}

const save_path_config = async () => {
    console.log(path_data.value);
    await invoke('save_path_config', { pathData: path_data.value });
    ElMessage({
        message: '配置文件已保存',
        type: 'success',
    })
}

const get_key_filter_data = async () => {
    const data = await invoke<Array<KeyFilterData>>('get_key_filter_data');
    key_data.value = data;
}

const save_key_filter_data = async () => {
    await invoke('save_key_filter_data', { keyFilterData: key_data.value })
    ElMessage({
        message: '配置文件已保存',
        type: 'success',
    })
}

const deleteIndexFileRow = (index: number) => {
    index_file_data.value?.splice(index, 1)
}

const deleteIndexWebPages = (index: number) => {
    index_web_pages_data.value?.splice(index, 1)
}

const deleteTargetPathRow = (index: number) => {
    path_data.value?.target_paths.splice(index, 1)
}

const deleteForbiddenPathRow = (index: number) => {
    path_data.value?.forbidden_paths.splice(index, 1)
}

const deleteForbiddenKeyRow = (index: number) => {
    path_data.value?.forbidden_key.splice(index, 1)
}

const deleteKeyFilterRow = (index: number) => {
    key_data.value?.splice(index, 1)
}

const add_target_path = () => {
    path_data.value?.target_paths.push("");
}

const add_forbidden_path = () => {
    path_data.value?.forbidden_paths.push("");
}

const add_forbidden_key = () => {
    path_data.value?.forbidden_key.push("");
}

const add_key_filter = () => {
    key_data.value?.push({ key: "", bias: 0, note: "" });
}

const add_index_file = () => {
    index_file_data.value?.push("");
}

const add_index_web_page = () => {
    index_web_pages_data.value?.push({ show_name: "", url: "" });
}

const get_remote_config_dir_path = async () => {
    const dir_path = await invoke<string>('get_remote_config_dir');
    remote_config_path_dir.value = dir_path;
}

const update_setting_window_info = async () => {
    await get_app_config();
    await get_path_config();
    await get_program_info();
    await get_key_filter_data();
    await get_index_web_pages();
    await get_index_file_data();
    await get_remote_config_dir_path();
}

onMounted(async () => {
    update_setting_window_info();
});

onUnmounted(() => {

})


</script>

<style></style>
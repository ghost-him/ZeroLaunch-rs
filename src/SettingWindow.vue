<template>
    <el-tabs v-model="activeName" class="demo-tabs" @tab-click="handleClick">
        <el-tab-pane label="程序设置" name="first">
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
                    <el-input-number v-model="config.search_result_count" step="1" />
                </el-form-item>

                <el-form-item label="自动刷新数据库的时间（分钟）">
                    <el-input-number v-model="config.auto_refresh_time" step="1" />
                </el-form-item>
                <el-button type="primary" @click="save_app_config">提交</el-button>
            </el-form>
        </el-tab-pane>
        <el-tab-pane label="自定义搜索路径" name="second">
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
                        <el-table-column label="目标路径" show-overflow-tooltip>
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
        <el-tab-pane label="关键字过滤设置" name="third">
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
        <el-tab-pane label="查看当前索引的所有的程序" name="fourth">
            <el-button class="mt-4" style="width: 100%" @click="get_program_info">
                刷新列表
            </el-button>
            <el-table :data="program_info" stripe style="width: 100%; height: 100%">
                <el-table-column label="程序名" show-overflow-tooltip>
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
                <el-table-column label="路径" show-overflow-tooltip>
                    <template #default="scope">
                        {{ program_info[scope.$index].path }}
                    </template>
                </el-table-column>
            </el-table>

        </el-tab-pane>
    </el-tabs>
</template>
<script lang="ts" setup>
import { ref, reactive, onMounted, onUnmounted } from 'vue'
import type { TabsPaneContext } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'

const activeName = ref('first')
const handleClick = (tab: TabsPaneContext, event: Event) => {
    console.log(tab, event)
}

// do not use same name with ref
const config = reactive({
    search_bar_placeholder: '',
    search_bar_no_result: '',
    is_auto_start: false,
    is_silent_start: false,

    search_result_count: 4,
    auto_refresh_time: 30,
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
}

const key_data = ref<Array<KeyFilterData>>([])
const program_info = ref<Array<ProgramInfo>>([])

const get_app_config = async () => {
    const loadedConfig = await invoke('get_app_config')

    Object.assign(config, loadedConfig)
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


onMounted(async () => {
    await get_app_config();
    await get_path_config();
    await get_program_info();
    await get_key_filter_data();
});

onUnmounted(() => {

})


</script>

<style></style>
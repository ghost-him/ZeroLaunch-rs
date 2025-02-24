<template>
    <el-tabs v-model="activeName" class="demo-tabs" drag>
        <el-tab-pane label="程序设置" name="a">
            <el-form label-width="auto" style="max-width: 600px">

                <el-form-item label="自定义搜索栏的提示文本">
                    <el-input v-model="config.app_config.search_bar_placeholder" placeholder="Hello, ZeroLaunch!"
                        @update:model-value="(val: string) => configStore.updateConfig({ app_config: { search_bar_placeholder: val } })" />
                </el-form-item>

                <el-form-item label="自定义搜索无结果的文本">
                    <el-input v-model="config.app_config.search_bar_no_result" placeholder="当前搜索无结果"
                        @update:model-value="(val: string) => configStore.updateConfig({ app_config: { search_bar_no_result: val } })" />
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

                <el-form-item label="设置选中项的背景颜色">
                    <el-color-picker v-model="config.ui_config.selected_item_color"
                        @update:model-value="(val: string) => configStore.updateConfig({ ui_config: { selected_item_color: val } })" />
                </el-form-item>

                <el-form-item label="设置选中项的字体颜色">
                    <el-color-picker v-model="config.ui_config.item_font_color"
                        @update:model-value="(val: string) => configStore.updateConfig({ ui_config: { item_font_color: val } })" />
                </el-form-item>

                <el-form-item label="选择背景图片">
                    <el-button type="primary" @click="select_background_picture">选择图片</el-button>
                    <el-button type="danger" @click="delete_background_picture">删除图片</el-button>
                </el-form-item>
                <el-button type="primary" @click="save_config">提交</el-button>
            </el-form>
        </el-tab-pane>
        <el-tab-pane label="自定义搜索路径" name="b">
            <el-tabs tab-position="left" style="height: 100% " class="demo-tabs">
                <el-tab-pane label="设置遍历路径">
                    <el-table :data="config.program_manager_config.loader.target_paths" stripe
                        style="width: 100%; height: 100%">
                        <el-table-column label="目标路径" show-overflow-tooltip>
                            <template #default="scope">
                                <el-input v-model="config.program_manager_config.loader.target_paths[scope.$index]"
                                    size="small" placeholder="请输入目标路径"
                                    @input="updateTargetPath(scope.$index, $event)"></el-input>
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

                    <el-button class="mt-4" style="width: 100%" @click="addTargetPath">
                        Add Item
                    </el-button>
                </el-tab-pane>

                <el-tab-pane label="设置屏蔽路径">
                    <el-table :data="forbidden_paths" stripe style="width: 100%; height: 100%">
                        <el-table-column label="目标屏蔽路径" show-overflow-tooltip>
                            <template #default="{ $index }">
                                <el-input v-model="forbidden_paths[$index]" size="small" placeholder="请输入目标路径"
                                    @input="updateForbiddenPaths"></el-input>
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
                                <el-input v-model="forbidden_program_key[$index]" size="small" placeholder="请输入目标关键字"
                                    @input="updateForbiddenProgramKey"></el-input>
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

                <el-tab-pane label="额外设置">
                    <el-form-item label="扫描UWP应用">
                        <el-switch v-model="is_scan_uwp_programs" @change="updateIsScanUwpPrograms" />
                    </el-form-item>
                </el-tab-pane>
            </el-tabs>
            <el-button type="primary" @click="save_config">提交</el-button>
        </el-tab-pane>

        <el-tab-pane label="关键字过滤设置" name="c">
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
                Add Item
            </el-button>
            <el-button type="primary" @click="save_config">提交</el-button>
        </el-tab-pane>

        <el-tab-pane label="添加自定义索引文件" name="d" drag>
            <el-tabs tab-position="left" style="height: 100%" class="demo-tabs">
                <el-tab-pane label="索引文件">
                    <div class="mb-4">
                        <el-button type="primary" plain @click="handleSelectFile">选择一个文件</el-button>
                    </div>

                    <el-table :data="index_file_paths" stripe style="width: 100%; height: 100%">
                        <el-table-column label="目标路径" show-overflow-tooltip>
                            <template #default="scope">
                                <el-input v-model="index_file_paths[scope.$index]" size="small" placeholder="请输入目标路径"
                                    @input="updateIndexFilePaths"></el-input>
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
                    <el-button class="mt-4" style="width: 100%" @click="addIndexFile">
                        Add Item
                    </el-button>
                </el-tab-pane>

                <el-tab-pane label="索引网址">
                    <el-table :data="index_web_pages" stripe style="width: 100%; height: 100%">
                        <el-table-column label="关键字（用于搜索程序的匹配）" show-overflow-tooltip>
                            <template #default="scope">
                                <el-input v-model="index_web_pages[scope.$index][0]" size="small" placeholder="请输入关键字"
                                    @input="updateIndexWebPages"></el-input>
                            </template>
                        </el-table-column>
                        <el-table-column label="目标网站的地址" show-overflow-tooltip>
                            <template #default="scope">
                                <el-input v-model="index_web_pages[scope.$index][1]" size="small" placeholder="请输入目标路径"
                                    @input="updateIndexWebPages"></el-input>
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
                    <el-button class="mt-4" style="width: 100%" @click="addIndexWebPage">
                        Add Item
                    </el-button>
                </el-tab-pane>
            </el-tabs>
            <el-button type="primary" @click="save_config">提交</el-button>
        </el-tab-pane>
        <el-tab-pane label="其它设置" name="e">
            <el-form-item label="设置配置文件的保存地址">
                <el-button type="primary" @click="change_remote_config_path_dir"> 选择目标路径</el-button>
                <el-input v-model="remote_config_path_dir" placeholder="设置配置文件保存路径" />
            </el-form-item>
        </el-tab-pane>
        <el-tab-pane label="查看当前索引的所有程序" name="f">
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
        </el-tab-pane>
        <el-tab-pane label="关于" name="g" drag>
            项目地址： https://github.com/ghost-him/ZeroLaunch-rs
        </el-tab-pane>
    </el-tabs>
</template>
<script lang="ts" setup>
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { ElMessage } from 'element-plus';
import { open } from '@tauri-apps/plugin-dialog';
import { useConfigStore } from '../stores/config';
import { storeToRefs } from 'pinia';

const configStore = useConfigStore()

const { config } = storeToRefs(configStore)


const updateTargetPath = (index: number, value: string) => {
    const newTargetPaths = [...config.value.program_manager_config.loader.target_paths]
    newTargetPaths[index] = value
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
    const newTargetPaths = [...config.value.program_manager_config.loader.target_paths, ""]
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

const handleSelectFile = async () => {
    const filePath = await open({ canCreateDirectories: false, directory: false, multiple: false, title: "选择一个文件" })
    if (filePath) {
        index_file_paths.value = [...index_file_paths.value, filePath as string]
    }
}

const deleteIndexFileRow = (index: number) => {
    index_file_paths.value = index_file_paths.value.filter((_, i) => i !== index)
}

const addIndexFile = () => {
    index_file_paths.value = [...index_file_paths.value, ""]
}

const deleteIndexWebPages = (index: number) => {
    index_web_pages.value = index_web_pages.value.filter((_, i) => i !== index)
}

const addIndexWebPage = () => {
    index_web_pages.value = [...index_web_pages.value, ["", ""]]
}

const updateIndexFilePaths = () => {
    configStore.updateConfig({
        program_manager_config: {
            loader: { index_file_paths: index_file_paths.value }
        }
    })
}

const updateIndexWebPages = () => {
    configStore.updateConfig({
        program_manager_config: {
            loader: { index_web_pages: index_web_pages.value }
        }
    })
}
const activeName = ref('a')
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

const deleteForbiddenPath = (index: number) => {
    const newPaths = [...forbidden_paths.value]
    newPaths.splice(index, 1)
    forbidden_paths.value = newPaths
}

const addForbiddenPath = () => {
    forbidden_paths.value = [...forbidden_paths.value, ""]
}

const deleteForbiddenKey = (index: number) => {
    const newKeys = [...forbidden_program_key.value]
    newKeys.splice(index, 1)
    forbidden_program_key.value = newKeys
}

const addForbiddenKey = () => {
    forbidden_program_key.value = [...forbidden_program_key.value, ""]
}

const updateForbiddenPaths = () => {
    configStore.updateConfig({
        program_manager_config: {
            loader: { forbidden_paths: forbidden_paths.value }
        }
    })
}

const updateForbiddenProgramKey = () => {
    configStore.updateConfig({
        program_manager_config: {
            loader: { forbidden_program_key: forbidden_program_key.value }
        }
    })
}

const updateIsScanUwpPrograms = () => {
    configStore.updateConfig({
        program_manager_config: {
            loader: { is_scan_uwp_programs: is_scan_uwp_programs.value }
        }
    })
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
            configStore.loadConfig();
            // 在这里处理选中的文件夹路径
        } else {
            console.log('没有选择文件夹');
        }
    } catch (err) {
        console.error('选择文件夹时出错:', err);
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


const save_config = async () => {
    await configStore.syncConfig()
    ElMessage({
        message: '配置文件已保存',
        type: 'success',
    })
}

onMounted(async () => {
    await configStore.loadConfig()
})


onUnmounted(() => {

})


</script>

<style></style>
<template>
    <el-tabs style="height: 100%">
        <el-tab-pane label="设置搜索路径" style="height: 100%">
            <div class="path-config-container">
                <!-- 左侧路径列表 -->
                <div class="path-list-section" :class="{ 'drag-over': isDragOver }">
                    <div class="section-header">
                        <h3>目录列表</h3>
                        <el-tooltip class="box-item" effect="dark" content="支持拖放文件与文件夹来添加搜索路径">
                            <el-icon class="el-question-icon">
                                <QuestionFilled />
                            </el-icon>
                        </el-tooltip>
                        <span>共有 {{ targetPaths.length }} 条记录</span>
                    </div>

                    <el-scrollbar>
                        <div v-for="(path, index) in targetPaths" :key="index" class="path-item"
                            :class="{ 'active': selectedPathIndex === index }" @click="selectPath(index)">
                            <div class="path-text">{{ path.root_path || '未设置路径' }}</div>
                            <div class="path-actions">
                                <el-button type="danger" size="small" circle @click.stop="deleteTargetPathRow(index)"
                                    title="删除">
                                    <el-icon>
                                        <Delete />
                                    </el-icon>
                                </el-button>
                            </div>
                        </div>
                    </el-scrollbar>

                    <el-button type="primary" @click="addTargetPath" class="add-path-btn" style="margin-top: 10px;">
                        <el-icon>
                            <Plus />
                        </el-icon> 添加路径
                    </el-button>
                </div>

                <!-- 右侧详细配置 -->
                <div class="path-detail-section" v-if="selectedPathIndex !== null">
                    <div class="detail-form">
                        <div class="form-row">
                            <div class="form-label">目标路径:</div>
                            <el-input v-model="currentPath.root_path" placeholder="请输入目标路径"
                                @change="updateCurrentPath"></el-input>
                        </div>

                        <div class="form-row">
                            <div class="form-label">搜索深度:</div>
                            <el-input-number v-model="currentPath.max_depth" :min="1" :precision="0"
                                @change="updateCurrentPath"></el-input-number>
                            <el-tooltip class="box-item" effect="dark"
                                content="搜索深度为2表示：搜索当前文件夹下的所有文件 以及 下一层子文件夹中的所有的文件">
                                <el-icon class="el-question-icon">
                                    <QuestionFilled />
                                </el-icon>
                            </el-tooltip>
                        </div>

                        <div class="form-row">
                            <div class="form-label">匹配类型:</div>
                            <el-select v-model="currentPath.pattern_type" placeholder="请选择匹配类型"
                                @change="updateCurrentPath">
                                <el-option label="正则表达式" value="Regex"></el-option>
                                <el-option label="通配符" value="Wildcard"></el-option>
                            </el-select>
                        </div>

                        <!-- 允许的扩展名表格 -->
                        <div class="form-section">
                            <div class="section-header">
                                <div class="form-label">允许的扩展名:</div>
                            </div>
                            <div class="extensions-table">
                                <el-table :data="extensionsTableData" size="small" height="150">
                                    <el-table-column prop="value" label="扩展名">
                                        <template #default="{ row, $index }">
                                            <el-input v-model="row.value" size="small" placeholder="输入扩展名"
                                                @change="updateExtension($index, row.value)"></el-input>
                                        </template>
                                    </el-table-column>
                                    <el-table-column width="60">
                                        <template #default="{ $index }">
                                            <el-button type="danger" size="small" circle
                                                @click="removeExtension($index)">
                                                <el-icon>
                                                    <Delete />
                                                </el-icon>
                                            </el-button>
                                        </template>
                                    </el-table-column>
                                </el-table>
                                <div class="table-actions">
                                    <el-button type="primary" size="small" @click="addExtension">
                                        <el-icon>
                                            <Plus />
                                        </el-icon> 添加
                                    </el-button>
                                    <el-dropdown @command="addCommonExtension"
                                        v-if="currentPath.pattern_type === 'Wildcard'">
                                        <el-button size="small">
                                            常用扩展名 <el-icon>
                                                <ArrowDown />
                                            </el-icon>
                                        </el-button>
                                        <template #dropdown>
                                            <el-dropdown-menu>
                                                <el-dropdown-item v-for="ext in commonExtensions" :key="ext"
                                                    :command="ext">
                                                    {{ ext }}
                                                </el-dropdown-item>
                                            </el-dropdown-menu>
                                        </template>
                                    </el-dropdown>
                                </div>
                            </div>
                        </div>

                        <!-- 排除关键词表格 -->
                        <div class="form-section">
                            <div class="section-header">
                                <div class="form-label">排除关键词(大小写不敏感):</div>
                            </div>
                            <div class="keywords-table">
                                <el-table :data="keywordsTableData" size="small" height="150">
                                    <el-table-column prop="value" label="关键词">
                                        <template #default="{ row, $index }">
                                            <el-input v-model="row.value" size="small" placeholder="输入关键词"
                                                @change="updateKeyword($index, row.value)"></el-input>
                                        </template>
                                    </el-table-column>
                                    <el-table-column width="60">
                                        <template #default="{ $index }">
                                            <el-button type="danger" size="small" circle @click="removeKeyword($index)">
                                                <el-icon>
                                                    <Delete />
                                                </el-icon>
                                            </el-button>
                                        </template>
                                    </el-table-column>
                                </el-table>
                                <div class="table-actions">
                                    <el-button type="primary" size="small" @click="addKeyword">
                                        <el-icon>
                                            <Plus />
                                        </el-icon> 添加
                                    </el-button>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>

                <!-- 未选择路径时的提示 -->
                <div class="path-detail-section empty-state" v-else>
                    <el-empty description="请选择或添加一个路径进行配置"></el-empty>
                </div>
            </div>
        </el-tab-pane>
        <el-tab-pane label="设置屏蔽路径" style="height: 100%">
            <div style="display: flex; flex-direction: column; height: 100%;">
                <el-button class="mt-4" style="width: 100%; flex-shrink: 0;" @click="addForbiddenPath">
                    添加项目
                </el-button>
                <el-table :data="forbidden_paths" stripe
                    style="width: 100%;flex-grow: 1; height: 0; min-height: 0; margin-top: 10px;">
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
            </div>
        </el-tab-pane>

        <el-tab-pane label="设置固定偏移量" style="height: 100%">
            <div style="display: flex; flex-direction: column; height: 100%;">
                <el-button class="mt-4" style="width: 100%; flex-shrink: 0;" @click="addKeyFilter">
                    添加项目
                </el-button>
                <el-table :data="keyFilterData" stripe
                    style="width: 100%;flex-grow: 1; height: 0; min-height: 0; margin-top: 10px;">
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
            </div>
        </el-tab-pane>
        <el-tab-pane label="额外设置" style="height: 100%;overflow:auto">
            <el-form-item label="扫描UWP应用">
                <el-switch v-model="config.program_manager_config.loader.is_scan_uwp_programs" @change="(val: boolean) =>
                    configStore.updateConfig({
                        program_manager_config: {
                            loader: { is_scan_uwp_programs: val }
                        }
                    })
                " />
            </el-form-item>

            <el-form-item label="启用联网加载网页图标">
                <el-switch v-model="config.program_manager_config.image_loader.enable_online" @change="(val: boolean) =>
                    configStore.updateConfig({
                        program_manager_config: {
                            image_loader: { enable_online: val }
                        }
                    })
                " />
                <el-tooltip class="box-item" effect="dark" content="开启后，即可加载网址的图标，成功获取后会替代默认的图标">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>

            <el-form-item label="启用图标缓存">
                <el-switch v-model="config.program_manager_config.image_loader.enable_icon_cache" @change="(val: boolean) =>
                    configStore.updateConfig({
                        program_manager_config: {
                            image_loader: { enable_icon_cache: val }
                        }
                    })
                " />
                <el-tooltip class="box-item" effect="dark" content="为每个程序都在本地保存一份图标，这可以加速图标的加载速度">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>
            <el-button type="primary" @click="openIconCacheDir">
                打开图标缓存文件夹
            </el-button>
        </el-tab-pane>
    </el-tabs>
</template>

<script lang="ts" setup>
import { useRemoteConfigStore } from '../stores/remote_config';
import { storeToRefs } from 'pinia';
import { DirectoryConfig } from '../api/remote_config_types';

const configStore = useRemoteConfigStore()
const { config } = storeToRefs(configStore)
import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue'
import { Delete, Plus, ArrowDown, QuestionFilled } from '@element-plus/icons-vue'
import type { ComputedRef, Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core';
import { listen, TauriEvent, UnlistenFn } from '@tauri-apps/api/event';
import { DragDropEvent } from '@tauri-apps/api/webview';
import { ElMessage } from 'element-plus';

// 表格数据项接口
interface TableItem {
    value: string;
}

/**
 * 计算属性：获取和设置目标路径配置
 */
const targetPaths: ComputedRef<DirectoryConfig[]> = computed({
    get: () => config.value.program_manager_config.loader.target_paths,
    set: (value: DirectoryConfig[]) => {
        configStore.updateConfig({
            program_manager_config: {
                loader: { target_paths: value }
            }
        })
    }
})

// 当前选中的路径索引
const selectedPathIndex: Ref<number | null> = ref(null)

// 当前编辑的路径配置
const currentPath: Ref<DirectoryConfig> = ref({
    root_path: '',
    max_depth: 2,
    pattern: [],
    pattern_type: 'Wildcard',
    excluded_keywords: []
})

// 扩展名和关键词的表格数据
const extensionsTableData: Ref<TableItem[]> = ref([])
const keywordsTableData: Ref<TableItem[]> = ref([])

// 常用扩展名列表
const commonExtensions: string[] = [
    '*.lnk', '*.url', '*.exe', '*.pdf', '*.txt', '*.png', '*.md'
]

/**
 * 选择路径
 * @param index - 要选择的路径索引
 */
const selectPath = (index: number): void => {
    selectedPathIndex.value = index
    // 深拷贝当前选中的路径配置
    currentPath.value = JSON.parse(JSON.stringify(targetPaths.value[index]))

    // 更新表格数据
    console.log("刷新页面")
    extensionsTableData.value = currentPath.value.pattern.map(ext => ({ value: ext }))
    console.log(extensionsTableData.value)
    keywordsTableData.value = currentPath.value.excluded_keywords.map(keyword => ({ value: keyword }))
}

/**
 * 更新当前路径配置到存储
 */
const updateCurrentPath = (): void => {
    if (selectedPathIndex.value !== null) {
        // 确保数组属性正确
        currentPath.value.pattern = extensionsTableData.value.map(item => item.value)
        currentPath.value.excluded_keywords = keywordsTableData.value.map(item => item.value)

        // 更新到 targetPaths
        const newTargetPaths: DirectoryConfig[] = [...targetPaths.value]
        newTargetPaths[selectedPathIndex.value] = currentPath.value

        configStore.updateConfig({
            program_manager_config: {
                loader: {
                    target_paths: newTargetPaths
                }
            }
        })
    }
}

//添加新的扩展名
const addExtension = (): void => {
    extensionsTableData.value.push({ value: '' })
    updateCurrentPath()
}

/**
 * 更新指定索引的扩展名
 * @param index - 要更新的扩展名索引
 * @param value - 新的扩展名值
 */
const updateExtension = (index: number, value: string): void => {
    extensionsTableData.value[index].value = value
    updateCurrentPath()
}

/**
 * 删除指定索引的扩展名
 * @param index - 要删除的扩展名索引
 */
const removeExtension = (index: number): void => {
    extensionsTableData.value.splice(index, 1)
    updateCurrentPath()
}

/**
 * 添加常用扩展名
 * @param ext - 要添加的常用扩展名
 */
const addCommonExtension = (ext: string): void => {
    // 检查是否已存在
    if (!extensionsTableData.value.some(item => item.value === ext)) {
        extensionsTableData.value.push({ value: ext })
        updateCurrentPath()
    }
}

/**
 * 添加新的关键词
 */
const addKeyword = (): void => {
    keywordsTableData.value.push({ value: '' })
    updateCurrentPath()
}

/**
 * 更新指定索引的关键词
 * @param index - 要更新的关键词索引
 * @param value - 新的关键词值
 */
const updateKeyword = (index: number, value: string): void => {
    keywordsTableData.value[index].value = value
    updateCurrentPath()
}

/**
 * 删除指定索引的关键词
 * @param index - 要删除的关键词索引
 */
const removeKeyword = (index: number): void => {
    keywordsTableData.value.splice(index, 1)
    updateCurrentPath()
}

/**
 * 添加新的目标路径
 */
const addTargetPath = (): void => {
    const newRow: DirectoryConfig = {
        root_path: '',
        max_depth: 2,
        pattern: ['*.lnk', '*.url', '*.exe'],
        pattern_type: 'Wildcard',
        excluded_keywords: []
    }

    const newTargetPaths: DirectoryConfig[] = [...targetPaths.value, newRow]
    configStore.updateConfig({
        program_manager_config: {
            loader: {
                target_paths: newTargetPaths
            }
        }
    })

    // 选中新添加的路径
    setTimeout(() => {
        selectPath(newTargetPaths.length - 1)
    }, 0)
}

/**
 * 删除指定索引的目标路径
 * @param index - 要删除的路径索引
 */
const deleteTargetPathRow = (index: number): void => {
    const newTargetPaths: DirectoryConfig[] = targetPaths.value.filter((_, i) => i !== index)
    configStore.updateConfig({
        program_manager_config: {
            loader: {
                target_paths: newTargetPaths
            }
        }
    })

    // 如果删除的是当前选中的路径，则清除选择
    if (selectedPathIndex.value === index) {
        selectedPathIndex.value = null
    } else if (selectedPathIndex.value !== null && selectedPathIndex.value > index) {
        // 如果删除的是当前选中路径之前的路径，则索引减1
        selectedPathIndex.value--
    }
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

const openIconCacheDir = async () => {
    await invoke('command_open_icon_cache_dir');
}

let unlisten: Array<UnlistenFn | null> = [];

const isDragOver = ref<boolean>(false);


/**
 * 添加或更新目标路径配置 (核心逻辑)
 * @param configToAddOrUpdate - 要添加或用于更新的配置
 * @param existingIndex - 如果是更新，则为现有配置的索引；如果是添加，则为 -1
 */
const addOrUpdateTargetPath = (configToAddOrUpdate: DirectoryConfig, existingIndex: number = -1): void => {
    const newTargetPaths: DirectoryConfig[] = JSON.parse(JSON.stringify(targetPaths.value)); // 深拷贝当前列表

    if (existingIndex !== -1) {
        // --- 更新现有路径 ---
        // 通常是添加 pattern
        const existingPath = newTargetPaths[existingIndex];
        // 合并 pattern，去重
        const newPatterns = Array.from(new Set([...existingPath.pattern, ...configToAddOrUpdate.pattern]));
        existingPath.pattern = newPatterns;
        ElMessage({ type: 'success', message: `已更新路径 ${existingPath.root_path} 的模式` });
    } else {
        // --- 添加新路径 ---
        // 检查 root_path 是否重复
        const exists = newTargetPaths.some(p => p.root_path === configToAddOrUpdate.root_path);
        if (exists) {
            ElMessage({ type: 'warning', message: `路径 ${configToAddOrUpdate.root_path} 已存在，请检查` });
            return; // 如果已存在，则不添加（拖拽文件到已有目录时，应走上面的更新逻辑）
        }
        newTargetPaths.push(configToAddOrUpdate);
        ElMessage({ type: 'success', message: `已添加路径: ${configToAddOrUpdate.root_path}` });
    }

    // 使用最终确定的新列表更新 store
    configStore.updateConfig({
        program_manager_config: {
            loader: {
                target_paths: newTargetPaths
            }
        }
    });
    // 如果更新的是当前选中的路径，需要刷新右侧表单
    if (selectedPathIndex.value === existingIndex) {
        selectPath(existingIndex); // 重新加载选中项数据到 currentPath 和表格
    }
}


// 后端命令返回的数据的接口
interface PathInfo {
    path_type: "file" | "directory" | "error";
    original_path: string;
    parent_path?: string;
    filename?: string;
    error_message?: string;
}

/**
 * 处理 drop 事件。
 * @param payload - 来自 Tauri drop 事件的有效负载。
 */
const handleFileDrop = async (payload: DragDropEvent) => {
    isDragOver.value = false; // 重置视觉指示器
    payload.type = "enter";
    if (payload.type !== "enter") {
        return;
    }

    isDragOver.value = false; // 重置拖放视觉指示

    if (!payload.paths || payload.paths.length === 0) {
        return; // 没有路径，直接返回
    }

    for (const droppedPath of payload.paths) {
        try {
            // 调用后端命令获取路径信息
            const info = await invoke<PathInfo>('command_get_path_info', { pathStr: droppedPath });

            if (info.path_type === 'directory') {
                // --- 处理拖入的文件夹 ---
                const newDirConfig: DirectoryConfig = {
                    root_path: info.original_path,
                    max_depth: 2, // 文件夹默认深度 2
                    pattern: ['*.lnk', '*.url', '*.exe'], // 默认通配符
                    pattern_type: 'Wildcard',
                    excluded_keywords: ['帮助', 'help', 'uninstall', '卸载', 'zerolaunch-rs']
                };
                // 使用核心函数添加（会自动检查重复的 root_path）
                addOrUpdateTargetPath(newDirConfig, -1); // 索引 -1 表示尝试添加新条目

            } else if (info.path_type === 'file' && info.parent_path && info.filename) {
                // --- 处理拖入的文件 ---
                const parentPath = info.parent_path;
                const filenamePattern = info.filename; // 文件名作为通配符

                // 查找父目录是否已存在于 targetPaths 中
                const existingIndex = targetPaths.value.findIndex(p => p.root_path === parentPath);

                if (existingIndex !== -1) {
                    // 父目录已存在，更新现有条目
                    const existingPath = targetPaths.value[existingIndex];

                    // 检查文件名模式是否已存在
                    if (!existingPath.pattern.includes(filenamePattern)) {
                        // 创建一个只包含新模式的临时配置对象，用于更新
                        const updatePayload: DirectoryConfig = {
                            root_path: parentPath, // root_path 必须匹配用于查找
                            pattern: [filenamePattern], // 只包含要添加的新模式
                            // 其他字段可以不传或传空，因为 addOrUpdateTargetPath 会合并 pattern
                            max_depth: existingPath.max_depth, // 保持原有深度
                            pattern_type: existingPath.pattern_type, // 保持原有类型
                            excluded_keywords: existingPath.excluded_keywords // 保持原有排除项
                        };
                        // 调用核心函数更新，传入找到的索引
                        addOrUpdateTargetPath(updatePayload, existingIndex);
                    } else {
                        ElMessage({ type: 'info', message: `文件模式 '${filenamePattern}' 已存在于路径: ${parentPath}` });
                    }
                } else {
                    // 父目录不存在，创建新条目
                    const newFileConfig: DirectoryConfig = {
                        root_path: parentPath,
                        max_depth: 1, // 文件所在目录默认深度 1
                        pattern: [filenamePattern], // 模式为该文件名
                        pattern_type: 'Wildcard', // 默认为通配符
                        excluded_keywords: ['帮助', 'help', 'uninstall', '卸载', 'zerolaunch-rs']
                    };
                    // 调用核心函数添加新条目
                    addOrUpdateTargetPath(newFileConfig, -1);
                }

            } else if (info.error_message) {
                // 处理后端返回的错误
                console.error(`处理路径 ${info.original_path} 出错: ${info.error_message}`);
                ElMessage({ type: 'error', message: `处理路径 ${info.original_path} 出错: ${info.error_message}` });
            } else {
                // 处理未知类型或其他情况
                console.warn(`不支持的路径类型或信息缺失: ${info.original_path}`);
                ElMessage({ type: 'warning', message: `不支持的路径或信息缺失: ${info.original_path}` });
            }

        } catch (error) {
            // 处理调用 invoke 时的前端错误
            console.error(`调用 get_path_info 处理 ${droppedPath} 失败:`, error);
            ElMessage({ type: 'error', message: `无法检查路径 ${droppedPath}` });
        }
    } // end for loop
};

onMounted(async () => {
    unlisten.push(await listen<DragDropEvent>(TauriEvent.DRAG_DROP, (e) => {
        handleFileDrop(e.payload);
    }))
    unlisten.push(await listen(TauriEvent.DRAG_LEAVE, () => {
        isDragOver.value = false;
        console.log("离开");
    }))
    unlisten.push(await listen(TauriEvent.DRAG_ENTER, () => {
        isDragOver.value = true;
        console.log("进入");
    }))
})

onUnmounted(async () => {
    unlisten.forEach(unlistenFn => {
        if (unlistenFn) unlistenFn();
    });
    unlisten = [];
})

</script>

<style scoped>
.drag-over {
    background-color: #e0e0e0;
    border-color: #bbb;
}

.path-config-container {
    display: flex;
    gap: 20px;
    height: 100%;
    box-sizing: border-box;
}

.path-list-section {
    font-size: 14px;
    width: 150px;
    min-width: 250px;
    display: flex;
    flex-direction: column;
    border: 1px solid #e4e7ed;
    border-radius: 4px;
}

.path-detail-section {
    flex: 1;
    padding: 15px;
    height: 100%;
    border: 1px solid #e4e7ed;
    border-radius: 4px;
    overflow: auto;
    box-sizing: border-box;
}

.section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px;
    border-bottom: 1px solid #e4e7ed;
}

.section-header h3 {
    margin: 0;
    font-size: 16px;
}

.path-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px;
    border-bottom: 1px solid #f0f0f0;
    cursor: pointer;
}

.path-item:hover {
    background-color: #f5f7fa;
}

.path-item.active {
    background-color: #ecf5ff;
}

.path-text {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.path-actions {
    opacity: 0;
    transition: opacity 0.2s;
}

.path-item:hover .path-actions {
    opacity: 1;
}

.add-path-btn {
    margin: 10px;
    margin-top: auto;
}

.detail-form {
    display: flex;
    flex-direction: column;
    gap: 15px;
    overflow: auto;
    height: 100%;
}

.form-row {
    display: flex;
    align-items: center;
    gap: 10px;
}

.form-label {
    min-width: 80px;
    font-weight: 500;
}

.form-section {
    border: 1px solid #e4e7ed;
    border-radius: 4px;
    margin-top: 10px;
}

.extensions-table,
.keywords-table {
    padding: 10px;
}

.table-actions {
    display: flex;
    justify-content: flex-start;
    gap: 10px;
    margin-top: 10px;
}

.empty-state {
    display: flex;
    justify-content: center;
    align-items: center;
}

.el-question-icon {
    margin-left: 8px;
}
</style>

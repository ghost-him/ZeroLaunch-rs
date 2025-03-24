<template>
    <el-tabs style="height: 100%">
        <el-tab-pane label="设置搜索路径" style="height: 100%">
            <div class="path-config-container">
                <!-- 左侧路径列表 -->
                <div class="path-list-section">
                    <div class="section-header">
                        <h3>目录列表</h3>
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
                                <div class="form-label">排除关键词:</div>
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
            <el-button class="mt-4" style="width: 100%" @click="addForbiddenPath">
                添加项目
            </el-button>
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

        </el-tab-pane>

        <el-tab-pane label="设置固定偏移量" style="height: 100%">
            <el-button class="mt-4" style="width: 100%" @click="addKeyFilter">
                添加项目
            </el-button>
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

        </el-tab-pane>
        <el-tab-pane label="额外设置" style="height: 100%">
            <el-form-item label="扫描UWP应用">
                <el-switch v-model="is_scan_uwp_programs" @change="updateIsScanUwpPrograms" />
            </el-form-item>
        </el-tab-pane>
    </el-tabs>
</template>

<script lang="ts" setup>
import { useRemoteConfigStore } from '../stores/remote_config';
import { storeToRefs } from 'pinia';
import { DirectoryConfig } from '../api/remote_config_types';

const configStore = useRemoteConfigStore()
const { config } = storeToRefs(configStore)
import { computed, ref } from 'vue'
import { Delete, Plus, ArrowDown, QuestionFilled } from '@element-plus/icons-vue'
import type { ComputedRef, Ref } from 'vue'

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
    extensionsTableData.value = currentPath.value.pattern.map(ext => ({ value: ext }))
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
</script>

<style scoped>
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

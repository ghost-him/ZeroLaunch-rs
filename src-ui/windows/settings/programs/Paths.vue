<template>
    <div class="settings-page">
        <div class="path-config-container">
            <!-- 左侧路径列表 -->
            <div class="path-list-section" :class="{ 'drag-over': isDragOver }">
                <div class="section-header">
                    <h3>{{ t('program_index.directory_list') }}</h3>
                    <el-tooltip class="box-item" effect="dark" :content="t('program_index.drag_drop_tooltip')">
                        <el-icon class="el-question-icon">
                            <QuestionFilled />
                        </el-icon>
                    </el-tooltip>
                    <span>{{ t('program_index.total_records', { count: targetPaths.length }) }}</span>
                </div>

                <el-scrollbar>
                    <div v-for="(path, index) in targetPaths" :key="index" class="path-item"
                        :class="{ 'active': selectedPathIndex === index }" @click="selectPath(index)">
                        <div class="path-text">{{ path.root_path || t('program_index.no_path_set') }}</div>
                        <div class="path-actions">
                            <el-button type="danger" size="small" circle @click.stop="deleteTargetPathRow(index)"
                                :title="t('program_index.delete')">
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
                    </el-icon> {{ t('program_index.add_path') }}
                </el-button>
            </div>

            <!-- 右侧详细配置 -->
            <div class="path-detail-section" v-if="selectedPathIndex !== null">
                <div class="detail-form">
                    <div class="form-row">
                        <div class="form-label">{{ t('program_index.target_path') }}:</div>
                        <el-input v-model="currentPath.root_path"
                            :placeholder="t('program_index.enter_target_path')"
                            @change="updateCurrentPath"
                            class="input-with-button"
                            clearable
                        >
                        <template #suffix>
                            <el-button
                            text
                            :icon="Folder"
                            @click="handleSelectFolder"
                            />
                        </template>
                        </el-input>
                    </div>

                    <div class="form-row">
                        <div class="form-label">{{ t('program_index.search_depth') }}:</div>
                        <el-input-number v-model="currentPath.max_depth" :min="1" :precision="0"
                            @change="updateCurrentPath"></el-input-number>
                        <el-tooltip class="box-item" effect="dark"
                            :content="t('program_index.search_depth_tooltip')">
                            <el-icon class="el-question-icon">
                                <QuestionFilled />
                            </el-icon>
                        </el-tooltip>
                    </div>

                    <div class="form-row">
                        <div class="form-label">{{ t('program_index.symlink_mode') }}:</div>
                        <el-select v-model="currentPath.symlink_mode"
                            :placeholder="t('program_index.symlink_mode_explicit')" 
                            @change="updateCurrentPath"
                            style="flex: 1;">
                            <el-option :label="t('program_index.symlink_mode_explicit')" value="ExplicitOnly"></el-option>
                            <el-option :label="t('program_index.symlink_mode_auto')" value="Auto"></el-option>
                        </el-select>
                        <el-tooltip class="box-item" effect="dark"
                            :content="t('program_index.symlink_mode_tooltip')">
                            <el-icon class="el-question-icon">
                                <QuestionFilled />
                            </el-icon>
                        </el-tooltip>
                    </div>

                    <div class="form-row">
                        <div class="form-label">{{ t('program_index.max_symlink_depth') }}:</div>
                        <el-input-number v-model="currentPath.max_symlink_depth" :min="1" :max="10" :precision="0"
                            @change="updateCurrentPath"></el-input-number>
                        <el-tooltip class="box-item" effect="dark"
                            :content="t('program_index.max_symlink_depth_tooltip')">
                            <el-icon class="el-question-icon">
                                <QuestionFilled />
                            </el-icon>
                        </el-tooltip>
                    </div>

                    <div class="form-row">
                        <div class="form-label">{{ t('program_index.match_type') }}:</div>
                        <el-select v-model="currentPath.pattern_type"
                            :placeholder="t('program_index.select_match_type')" 
                            @change="updateCurrentPath"
                            style="flex: 1;">
                            <el-option :label="t('program_index.regex')" value="Regex"></el-option>
                            <el-option :label="t('program_index.wildcard')" value="Wildcard"></el-option>
                        </el-select>
                    </div>

                    <!-- 允许的扩展名表格 -->
                    <div class="form-section">
                        <div class="section-header">
                            <div class="form-label">{{ t('program_index.allowed_extensions') }}:</div>
                        </div>
                        <div class="extensions-table">
                            <el-table :data="extensionsTableData" size="small" height="150">
                                <el-table-column prop="value" :label="t('program_index.extension')">
                                    <template #default="{ row, $index }">
                                        <el-input v-model="row.value" size="small"
                                            :placeholder="t('program_index.enter_extension')"
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
                                    </el-icon> {{ t('program_index.add') }}
                                </el-button>
                                <el-dropdown @command="addCommonExtension"
                                    v-if="currentPath.pattern_type === 'Wildcard'">
                                    <el-button size="small">
                                        {{ t('program_index.common_extensions') }} <el-icon>
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
                            <div class="form-label">{{ t('program_index.exclude_keywords') }}:</div>
                        </div>
                        <div class="keywords-table">
                            <el-table :data="keywordsTableData" size="small" height="150">
                                <el-table-column prop="value" :label="t('program_index.keyword')">
                                    <template #default="{ row, $index }">
                                        <el-input v-model="row.value" size="small"
                                            :placeholder="t('program_index.enter_keyword')"
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
                                    </el-icon> {{ t('program_index.add') }}
                                </el-button>
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            <!-- 未选择路径时的提示 -->
            <div class="path-detail-section empty-state" v-else>
                <el-empty :description="t('program_index.select_or_add_path')"></el-empty>
            </div>
        </div>
    </div>
</template>

<script lang="ts" setup>
import { computed, ref, onMounted, onUnmounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRemoteConfigStore } from '../../../stores/remote_config';
import { storeToRefs } from 'pinia';
import { DirectoryConfig } from '../../../api/remote_config_types';
import { QuestionFilled, Delete, Plus, Folder, ArrowDown } from '@element-plus/icons-vue';
import { open } from '@tauri-apps/plugin-dialog';
import { homeDir } from '@tauri-apps/api/path';
import { invoke } from '@tauri-apps/api/core';
import { listen, TauriEvent, UnlistenFn } from '@tauri-apps/api/event';
import { ElMessage } from 'element-plus';

const { t } = useI18n();
const configStore = useRemoteConfigStore();
const { config } = storeToRefs(configStore);

interface TableItem {
    value: string;
}

const targetPaths = computed({
    get: () => config.value.program_manager_config.loader.target_paths,
    set: (value: DirectoryConfig[]) => {
        configStore.updateConfig({
            program_manager_config: {
                loader: { target_paths: value }
            }
        })
    }
})

const selectedPathIndex = ref<number | null>(null)
const currentPath = ref<DirectoryConfig>({
    root_path: '',
    max_depth: 2,
    pattern: [],
    pattern_type: 'Wildcard',
    excluded_keywords: [],
    symlink_mode: 'ExplicitOnly',
    max_symlink_depth: 4
})

const extensionsTableData = ref<TableItem[]>([])
const keywordsTableData = ref<TableItem[]>([])
const commonExtensions = ['*.lnk', '*.url', '*.exe', '*.pdf', '*.txt', '*.png', '*.md']
const isDragOver = ref(false);

const selectPath = (index: number) => {
    selectedPathIndex.value = index
    const selectedPath = targetPaths.value[index]
    currentPath.value = {
        ...JSON.parse(JSON.stringify(selectedPath)),
        symlink_mode: selectedPath.symlink_mode || 'ExplicitOnly',
        max_symlink_depth: selectedPath.max_symlink_depth || 4
    }
    extensionsTableData.value = currentPath.value.pattern.map(ext => ({ value: ext }))
    keywordsTableData.value = currentPath.value.excluded_keywords.map(keyword => ({ value: keyword }))
}

const updateCurrentPath = () => {
    if (selectedPathIndex.value !== null) {
        currentPath.value.pattern = extensionsTableData.value.map(item => item.value)
        currentPath.value.excluded_keywords = keywordsTableData.value.map(item => item.value)
        const newTargetPaths = [...targetPaths.value]
        newTargetPaths[selectedPathIndex.value] = currentPath.value
        configStore.updateConfig({
            program_manager_config: {
                loader: { target_paths: newTargetPaths }
            }
        })
    }
}

const handleSelectFolder = async () => {
    const folderSelected = await open({
        directory: true,
        multiple: false,
        defaultPath: await homeDir()
    })
    if (folderSelected) {
        const picked = Array.isArray(folderSelected) ? folderSelected[0] : folderSelected
        currentPath.value.root_path = picked as string
        updateCurrentPath()
    }
}

const addExtension = () => {
    extensionsTableData.value.push({ value: '' })
    updateCurrentPath()
}

const updateExtension = (index: number, value: string) => {
    extensionsTableData.value[index].value = value
    updateCurrentPath()
}

const removeExtension = (index: number) => {
    extensionsTableData.value.splice(index, 1)
    updateCurrentPath()
}

const addCommonExtension = (ext: string) => {
    if (!extensionsTableData.value.some(item => item.value === ext)) {
        extensionsTableData.value.push({ value: ext })
        updateCurrentPath()
    }
}

const addKeyword = () => {
    keywordsTableData.value.push({ value: '' })
    updateCurrentPath()
}

const updateKeyword = (index: number, value: string) => {
    keywordsTableData.value[index].value = value
    updateCurrentPath()
}

const removeKeyword = (index: number) => {
    keywordsTableData.value.splice(index, 1)
    updateCurrentPath()
}

const addTargetPath = () => {
    const newRow: DirectoryConfig = {
        root_path: '',
        max_depth: 2,
        pattern: ['*.lnk', '*.url', '*.exe'],
        pattern_type: 'Wildcard',
        excluded_keywords: [],
        symlink_mode: 'ExplicitOnly',
        max_symlink_depth: 4
    }
    const newTargetPaths = [...targetPaths.value, newRow]
    configStore.updateConfig({
        program_manager_config: {
            loader: { target_paths: newTargetPaths }
        }
    })
    setTimeout(() => {
        selectPath(newTargetPaths.length - 1)
    }, 0)
}

const deleteTargetPathRow = (index: number) => {
    const newTargetPaths = targetPaths.value.filter((_, i) => i !== index)
    configStore.updateConfig({
        program_manager_config: {
            loader: { target_paths: newTargetPaths }
        }
    })
    if (selectedPathIndex.value === index) {
        selectedPathIndex.value = null
    } else if (selectedPathIndex.value !== null && selectedPathIndex.value > index) {
        selectedPathIndex.value--
    }
}

// Drag and Drop Logic
interface PathInfo {
    path_type: "file" | "directory" | "error";
    original_path: string;
    parent_path?: string;
    filename?: string;
    error_message?: string;
}

interface DragDropPayload {
    paths: string[];
    position: { x: number; y: number };
}

const addOrUpdateTargetPath = (configToAddOrUpdate: DirectoryConfig, existingIndex: number = -1) => {
    const newTargetPaths = JSON.parse(JSON.stringify(targetPaths.value));
    if (existingIndex !== -1) {
        const existingPath = newTargetPaths[existingIndex];
        const newPatterns = Array.from(new Set([...existingPath.pattern, ...configToAddOrUpdate.pattern]));
        existingPath.pattern = newPatterns;
        ElMessage({ type: 'success', message: t('program_index.path_pattern_updated', { path: existingPath.root_path }) });
    } else {
        const exists = newTargetPaths.some((p: DirectoryConfig) => p.root_path === configToAddOrUpdate.root_path);
        if (exists) {
            ElMessage({ type: 'warning', message: t('program_index.path_already_exists', { path: configToAddOrUpdate.root_path }) });
            return;
        }
        newTargetPaths.push(configToAddOrUpdate);
        ElMessage({ type: 'success', message: t('program_index.path_added', { path: configToAddOrUpdate.root_path }) });
    }
    configStore.updateConfig({
        program_manager_config: {
            loader: { target_paths: newTargetPaths }
        }
    });
    if (selectedPathIndex.value === existingIndex) {
        selectPath(existingIndex);
    }
}

const handleFileDrop = async (payload: DragDropPayload) => {
    isDragOver.value = false;
    
    if (!payload.paths || payload.paths.length === 0) return;

    for (const droppedPath of payload.paths) {
        try {
            const info = await invoke<PathInfo>('command_get_path_info', { pathStr: droppedPath });
            if (info.path_type === 'directory') {
                const newDirConfig: DirectoryConfig = {
                    root_path: info.original_path,
                    max_depth: 2,
                    pattern: ['*.lnk', '*.url', '*.exe'],
                    pattern_type: 'Wildcard',
                    excluded_keywords: ['帮助', 'help', 'uninstall', '卸载', 'zerolaunch-rs'],
                    symlink_mode: 'ExplicitOnly',
                    max_symlink_depth: 4
                };
                addOrUpdateTargetPath(newDirConfig, -1);
            } else if (info.path_type === 'file' && info.parent_path && info.filename) {
                const parentPath = info.parent_path;
                const filenamePattern = info.filename;
                const existingIndex = targetPaths.value.findIndex(p => p.root_path === parentPath);
                if (existingIndex !== -1) {
                    const existingPath = targetPaths.value[existingIndex];
                    if (!existingPath.pattern.includes(filenamePattern)) {
                        const updatePayload: DirectoryConfig = {
                            root_path: parentPath,
                            pattern: [filenamePattern],
                            max_depth: existingPath.max_depth,
                            pattern_type: existingPath.pattern_type,
                            excluded_keywords: existingPath.excluded_keywords,
                            symlink_mode: existingPath.symlink_mode,
                            max_symlink_depth: existingPath.max_symlink_depth
                        };
                        addOrUpdateTargetPath(updatePayload, existingIndex);
                    } else {
                        ElMessage({ type: 'info', message: t('program_index.file_pattern_exists', { pattern: filenamePattern, path: parentPath }) });
                    }
                } else {
                    const newFileConfig: DirectoryConfig = {
                        root_path: parentPath,
                        max_depth: 1,
                        pattern: [filenamePattern],
                        pattern_type: 'Wildcard',
                        excluded_keywords: ['帮助', 'help', 'uninstall', '卸载', 'zerolaunch-rs'],
                        symlink_mode: 'ExplicitOnly',
                        max_symlink_depth: 4
                    };
                    addOrUpdateTargetPath(newFileConfig, -1);
                }
            } else if (info.error_message) {
                ElMessage({ type: 'error', message: t('program_index.process_path_error', { path: info.original_path, error: info.error_message }) });
            } else {
                ElMessage({ type: 'warning', message: t('program_index.unsupported_path_type', { path: info.original_path }) });
            }
        } catch (error) {
            ElMessage({ type: 'error', message: t('program_index.cannot_check_path', { path: droppedPath }) });
        }
    }
};

let unlisten: Array<UnlistenFn | null> = [];

onMounted(async () => {
    unlisten.push(await listen<DragDropPayload>(TauriEvent.DRAG_DROP, (e) => {
        handleFileDrop(e.payload);
    }))
    unlisten.push(await listen(TauriEvent.DRAG_LEAVE, () => {
        isDragOver.value = false;
    }))
    unlisten.push(await listen(TauriEvent.DRAG_ENTER, () => {
        isDragOver.value = true;
    }))
})

onUnmounted(() => {
    unlisten.forEach(fn => fn && fn());
    unlisten = [];
})
</script>

<style scoped>
.settings-page {
    padding: 20px;
    height: 100%;
    box-sizing: border-box;
}

.path-config-container {
    display: flex;
    gap: 20px;
    height: 100%;
    box-sizing: border-box;
}

.path-list-section {
    font-size: 14px;
    width: 250px;
    display: flex;
    flex-direction: column;
    border: 1px solid #e4e7ed;
    border-radius: 4px;
    background-color: #fff;
}

.path-detail-section {
    flex: 1;
    padding: 15px;
    height: 100%;
    border: 1px solid #e4e7ed;
    border-radius: 4px;
    overflow: auto;
    box-sizing: border-box;
    background-color: #fff;
}

.section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px;
    border-bottom: 1px solid #e4e7ed;
    background-color: #f5f7fa;
}

.section-header h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
}

.path-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px;
    border-bottom: 1px solid #f0f0f0;
    cursor: pointer;
    transition: background-color 0.2s;
}

.path-item:hover {
    background-color: #f5f7fa;
}

.path-item.active {
    background-color: #ecf5ff;
    border-right: 2px solid #409eff;
}

.path-text {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin-right: 10px;
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
}

.form-row {
    display: flex;
    align-items: center;
    gap: 10px;
}

.form-label {
    min-width: 120px;
    font-weight: 500;
    white-space: nowrap;
    color: #606266;
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
    color: #909399;
}

.drag-over {
    background-color: #ecf5ff;
    border: 2px dashed #409eff;
}
</style>

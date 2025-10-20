<template>
    <el-tabs style="height: 100%">
        <el-tab-pane :label="t('program_index.set_search_path')" style="height: 100%">
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
                                @change="updateCurrentPath"></el-input>
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
                            <div class="form-label">{{ t('program_index.match_type') }}:</div>
                            <el-select v-model="currentPath.pattern_type"
                                :placeholder="t('program_index.select_match_type')" @change="updateCurrentPath">
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
        </el-tab-pane>
        <el-tab-pane :label="t('program_index.set_blocked_paths')" style="height: 100%">
            <div style="display: flex; flex-direction: column; height: 100%;">
                <el-button class="mt-4" style="width: 100%; flex-shrink: 0;" @click="addForbiddenPath">
                    {{ t('program_index.add_item') }}
                </el-button>
                <el-table :data="forbidden_paths" stripe
                    style="width: 100%;flex-grow: 1; height: 0; min-height: 0; margin-top: 10px;">
                    <el-table-column :label="t('program_index.target_blocked_path')" show-overflow-tooltip>
                        <template #default="{ $index }">
                            <el-input v-model="forbidden_paths[$index]" size="small"
                                :placeholder="t('program_index.enter_target_path')"
                                @change="updateForbiddenPaths"></el-input>
                        </template>
                    </el-table-column>
                    <el-table-column fixed="right" :label="t('program_index.operation')" width="100">
                        <template #default="{ $index }">
                            <el-button link size="small" type="danger" @click="deleteForbiddenPath($index)">
                                {{ t('program_index.delete_row') }}
                            </el-button>
                        </template>
                    </el-table-column>
                </el-table>
            </div>
        </el-tab-pane>

        <el-tab-pane :label="t('program_index.set_fixed_offset')" style="height: 100%">
            <div style="display: flex; flex-direction: column; height: 100%;">
                <el-button class="mt-4" style="width: 100%; flex-shrink: 0;" @click="addKeyFilter">
                    {{ t('program_index.add_item') }}
                </el-button>
                <el-table :data="keyFilterData" stripe
                    style="width: 100%;flex-grow: 1; height: 0; min-height: 0; margin-top: 10px;">
                    <el-table-column :label="t('program_index.target_keyword')">
                        <template #default="{ row }">
                            <el-input v-model="row.key" size="small"
                                :placeholder="t('program_index.enter_target_keyword')"
                                @change="updateProgramBias(row)"></el-input>
                        </template>
                    </el-table-column>
                    <el-table-column :label="t('program_index.offset')" show-overflow-tooltip>
                        <template #default="{ row }">
                            <el-input-number v-model="row.bias" size="small"
                                :placeholder="t('program_index.enter_offset')"
                                @change="updateProgramBias(row)"></el-input-number>
                        </template>
                    </el-table-column>
                    <el-table-column :label="t('program_index.note')" show-overflow-tooltip>
                        <template #default="{ row }">
                            <el-input v-model="row.note" size="small" :placeholder="t('program_index.enter_note')"
                                @change="updateProgramBias(row)"></el-input>
                        </template>
                    </el-table-column>
                    <el-table-column fixed="right" :label="t('program_index.operation')" width="100">
                        <template #default="{ $index }">
                            <el-button link size="small" type="danger" @click="deleteKeyFilterRow($index)">
                                {{ t('program_index.delete_row') }}
                            </el-button>
                        </template>
                    </el-table-column>
                </el-table>
            </div>
        </el-tab-pane>

        <el-tab-pane :label="t('program_index.setting_alias')" style="height: 100%;overflow:auto">
            <div style="display: flex; flex-direction: column; height: 100%;">
                <el-button class="mt-4" style="width: 100%; flex-shrink: 0;" @click="refreshProgramInfo">
                    {{ t('program_index.click_refresh') }}
                </el-button>
                <el-table-v2 :columns="columns" :data="programInfoList" :width="1000" :height="600" fixed
                    style="width: 100%;flex-grow: 1; margin-top: 10px;" />
            </div>
        </el-tab-pane>

        <el-tab-pane :label="t('program_index.extra_settings')" style="height: 100%;overflow:auto">
            <el-form-item :label="t('program_index.change_search_algorithm')">
                <el-select v-model="config.program_manager_config.search_model" placeholder="standard"
                    style="width: 240px"
                    @change="(val: string) => configStore.updateConfig({ program_manager_config: { search_model: val } })">
                    <el-option v-for="item in search_model" :key="item.value" :label="item.label" :value="item.value" />
                </el-select>
                <el-tooltip class="box-item" effect="dark" :content="t('program_index.search_algorithm_tooltip')">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>

            <el-form-item :label="t('program_index.enable_lru_search_cache')">
                <el-switch v-model="config.program_manager_config.enable_lru_search_cache" @change="(val: boolean) =>
                    configStore.updateConfig({
                        program_manager_config: {
                            enable_lru_search_cache: val
                        }
                    })
                " />
                <el-tooltip class="box-item" effect="dark" :content="t('program_index.lru_cache_tooltip')">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>
            <p class="lru-cache-hint">{{ t('program_index.lru_cache_description') }}</p>

            <el-form-item v-if="config.program_manager_config.enable_lru_search_cache"
                :label="t('program_index.search_cache_capacity')">
                <el-input-number v-model="config.program_manager_config.search_cache_capacity" :min="1" :max="1000"
                    @change="(val: number) =>
                        configStore.updateConfig({
                            program_manager_config: {
                                search_cache_capacity: Math.max(1, val ?? 1)
                            }
                        })
                    "
                />
            </el-form-item>

            <!-- 语义搜索说明：仅在 semantic 模式下显示 -->
            <div v-if="config.program_manager_config.search_model === 'semantic'" class="semantic-section">
                <el-card shadow="never" class="semantic-card">
                    <p class="semantic-title">
                        {{ t('program_index.semantic_search_intro') || '使用EmbeddingGemma-300m实现的语义搜索，带来无与论比的搜索体验。' }}
                    </p>
                    <el-alert type="info" :closable="false" show-icon class="semantic-tip">
                        <template #title>
                            {{ t('program_index.semantic_tip_model') || '需要将模型单独下载到指定的目录中。' }}
                        </template>
                    </el-alert>
                    <el-alert type="warning" :closable="false" show-icon class="semantic-tip">
                        <template #title>
                            {{ t('program_index.semantic_tip_description') || '若补充完善描述信息，可让模型提供更准确的功能性搜索。' }}
                        </template>
                    </el-alert>
                    <el-alert type="error" :closable="false" show-icon class="semantic-tip">
                        <template #title>
                            {{ t('program_index.semantic_tip_performance') || '启用后在初始化与更新数据库时可能占用 GPU，并显著延长加载时间。' }}
                        </template>
                    </el-alert>
                    <div class="semantic-actions">
                        <el-button type="primary" @click="openModelFolder">{{ t('program_index.open_model_folder') || '打开模型文件夹 (TODO)' }}</el-button>
                    </div>
                </el-card>
            </div>

            <el-form-item :label="t('program_index.scan_uwp_apps')">
                <el-switch v-model="config.program_manager_config.loader.is_scan_uwp_programs" @change="(val: boolean) =>
                    configStore.updateConfig({
                        program_manager_config: {
                            loader: { is_scan_uwp_programs: val }
                        }
                    })
                " />
            </el-form-item>

            <el-form-item :label="t('program_index.enable_online_icon_loading')">
                <el-switch v-model="config.program_manager_config.image_loader.enable_online" @change="(val: boolean) =>
                    configStore.updateConfig({
                        program_manager_config: {
                            image_loader: { enable_online: val }
                        }
                    })
                " />
                <el-tooltip class="box-item" effect="dark" :content="t('program_index.online_icon_tooltip')">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>

            <el-form-item :label="t('program_index.enable_icon_cache')">
                <el-switch v-model="config.program_manager_config.image_loader.enable_icon_cache" @change="(val: boolean) =>
                    configStore.updateConfig({
                        program_manager_config: {
                            image_loader: { enable_icon_cache: val }
                        }
                    })
                " />
                <el-tooltip class="box-item" effect="dark" :content="t('program_index.icon_cache_tooltip')">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>
            <el-button type="primary" @click="openIconCacheDir">
                {{ t('program_index.open_icon_cache_folder') }}
            </el-button>

            <!-- 排序算法参数设置 -->
            <el-divider />
            <h3>{{ t('program_index.sorting_algorithm_settings') }}</h3>
            <el-alert type="warning" :closable="false" show-icon style="margin-bottom: 20px;">
                <template #title>
                    {{ t('program_index.sorting_params_warning') }}
                </template>
            </el-alert>

            <el-form-item :label="t('program_index.enable_sorting')">
                <el-switch v-model="config.program_manager_config.ranker.is_enable" @change="(val: boolean) =>
                    configStore.updateConfig({
                        program_manager_config: {
                            ranker: { is_enable: val }
                        }
                    })
                " />
                <el-tooltip class="box-item" effect="dark" :content="t('program_index.enable_sorting_tooltip')">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>

            <div v-if="config.program_manager_config.ranker.is_enable">
                <el-form-item :label="t('program_index.history_weight')">
                    <el-input-number v-model="config.program_manager_config.ranker.history_weight" :min="0" :max="10" :step="0.1" :precision="2"
                        @change="(val: number) =>
                            configStore.updateConfig({
                                program_manager_config: {
                                    ranker: { history_weight: val ?? 1.2 }
                                }
                            })
                        "
                    />
                    <el-tooltip class="box-item" effect="dark" placement="right" style="max-width: 400px;">
                        <template #content>
                            <div style="max-width: 400px;">{{ t('program_index.history_weight_description') }}</div>
                        </template>
                        <el-icon class="el-question-icon">
                            <QuestionFilled />
                        </el-icon>
                    </el-tooltip>
                </el-form-item>

                <el-form-item :label="t('program_index.recent_habit_weight')">
                    <el-input-number v-model="config.program_manager_config.ranker.recent_habit_weight" :min="0" :max="10" :step="0.1" :precision="2"
                        @change="(val: number) =>
                            configStore.updateConfig({
                                program_manager_config: {
                                    ranker: { recent_habit_weight: val ?? 2.5 }
                                }
                            })
                        "
                    />
                    <el-tooltip class="box-item" effect="dark" placement="right">
                        <template #content>
                            <div style="max-width: 400px;">{{ t('program_index.recent_habit_weight_description') }}</div>
                        </template>
                        <el-icon class="el-question-icon">
                            <QuestionFilled />
                        </el-icon>
                    </el-tooltip>
                </el-form-item>

                <el-form-item :label="t('program_index.temporal_weight')">
                    <el-input-number v-model="config.program_manager_config.ranker.temporal_weight" :min="0" :max="10" :step="0.1" :precision="2"
                        @change="(val: number) =>
                            configStore.updateConfig({
                                program_manager_config: {
                                    ranker: { temporal_weight: val ?? 0.8 }
                                }
                            })
                        "
                    />
                    <el-tooltip class="box-item" effect="dark" placement="right">
                        <template #content>
                            <div style="max-width: 400px;">{{ t('program_index.temporal_weight_description') }}</div>
                        </template>
                        <el-icon class="el-question-icon">
                            <QuestionFilled />
                        </el-icon>
                    </el-tooltip>
                </el-form-item>

                <el-form-item :label="t('program_index.query_affinity_weight')">
                    <el-input-number v-model="config.program_manager_config.ranker.query_affinity_weight" :min="0" :max="10" :step="0.1" :precision="2"
                        @change="(val: number) =>
                            configStore.updateConfig({
                                program_manager_config: {
                                    ranker: { query_affinity_weight: val ?? 3.5 }
                                }
                            })
                        "
                    />
                    <el-tooltip class="box-item" effect="dark" placement="right">
                        <template #content>
                            <div style="max-width: 400px;">{{ t('program_index.query_affinity_weight_description') }}</div>
                        </template>
                        <el-icon class="el-question-icon">
                            <QuestionFilled />
                        </el-icon>
                    </el-tooltip>
                </el-form-item>

                <el-form-item :label="t('program_index.query_affinity_time_decay')">
                    <el-input-number v-model="config.program_manager_config.ranker.query_affinity_time_decay" :min="0" :max="1000000" :step="3600"
                        @change="(val: number) =>
                            configStore.updateConfig({
                                program_manager_config: {
                                    ranker: { query_affinity_time_decay: val ?? 259200 }
                                }
                            })
                        "
                    />
                    <el-tooltip class="box-item" effect="dark" placement="right">
                        <template #content>
                            <div style="max-width: 400px;">{{ t('program_index.query_affinity_time_decay_description') }}</div>
                        </template>
                        <el-icon class="el-question-icon">
                            <QuestionFilled />
                        </el-icon>
                    </el-tooltip>
                </el-form-item>

                <el-form-item :label="t('program_index.temporal_decay')">
                    <el-input-number v-model="config.program_manager_config.ranker.temporal_decay" :min="0" :max="1000000" :step="3600"
                        @change="(val: number) =>
                            configStore.updateConfig({
                                program_manager_config: {
                                    ranker: { temporal_decay: val ?? 10800 }
                                }
                            })
                        "
                    />
                    <el-tooltip class="box-item" effect="dark" placement="right">
                        <template #content>
                            <div style="max-width: 400px;">{{ t('program_index.temporal_decay_description') }}</div>
                        </template>
                        <el-icon class="el-question-icon">
                            <QuestionFilled />
                        </el-icon>
                    </el-tooltip>
                </el-form-item>
            </div>
        </el-tab-pane>
    </el-tabs>

    <el-dialog v-if="editingProgram" v-model="dialogVisible"
        :title="t('settings.edit_program_alias', { name: editingProgram.name })" width="500">
        <div style="display: flex; flex-direction: column; gap: 10px;">
            <div v-for="(alias, index) in program_alias[editingProgram.path]" :key="index"
                style="display: flex; align-items: center; gap: 10px;">
                <el-input :model-value="alias" @update:modelValue="(newValue: string) => updateAliasInDialog(index, newValue)"
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
import { useRemoteConfigStore } from '../stores/remote_config';
import { storeToRefs } from 'pinia';
import { DirectoryConfig } from '../api/remote_config_types';

const { t } = useI18n();

const configStore = useRemoteConfigStore()
const { config } = storeToRefs(configStore)
import { computed, h, onMounted, onUnmounted, ref } from 'vue'
import { Delete, Plus, ArrowDown, QuestionFilled } from '@element-plus/icons-vue'
import type { ComputedRef, Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core';
import { listen, TauriEvent, UnlistenFn } from '@tauri-apps/api/event';
import { DragDropEvent } from '@tauri-apps/api/webview';
import { ElButton, ElMessage, ElTag } from 'element-plus';

const search_model = computed(() => [
    {
        value: 'standard',
        label: t('program_index.standard_search_algorithm'),
    },
    {
        value: 'skim',
        label: t('program_index.skim_matching_algorithm'),
    }, {
        value: 'launchy',
        label: t('program_index.launchyqt_algorithm'),
    }, {
        value: 'semantic',
    label: t('program_index.semantic_search_algorithm')
    }
])

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
    const newKey = t('program_index.enter_keyword_placeholder')
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

// 语义搜索说明采用内联模式，不再使用弹窗

const openModelFolder = async () => {
    await invoke('command_open_models_dir');
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
        ElMessage({ type: 'success', message: t('program_index.path_pattern_updated', { path: existingPath.root_path }) });
    } else {
        // --- 添加新路径 ---
        // 检查 root_path 是否重复
        const exists = newTargetPaths.some(p => p.root_path === configToAddOrUpdate.root_path);
        if (exists) {
            ElMessage({ type: 'warning', message: t('program_index.path_already_exists', { path: configToAddOrUpdate.root_path }) });
            return; // 如果已存在，则不添加（拖拽文件到已有目录时，应走上面的更新逻辑）
        }
        newTargetPaths.push(configToAddOrUpdate);
        ElMessage({ type: 'success', message: t('program_index.path_added', { path: configToAddOrUpdate.root_path }) });
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
                        ElMessage({ type: 'info', message: t('program_index.file_pattern_exists', { pattern: filenamePattern, path: parentPath }) });
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
                console.error(t('program_index.process_path_error', { path: info.original_path, error: info.error_message }));
                ElMessage({ type: 'error', message: t('program_index.process_path_error', { path: info.original_path, error: info.error_message }) });
            } else {
                // 处理未知类型或其他情况
                console.warn(t('program_index.unsupported_path_type', { path: info.original_path }));
                ElMessage({ type: 'warning', message: t('program_index.unsupported_path_type', { path: info.original_path }) });
            }

        } catch (error) {
            // 处理调用 invoke 时的前端错误
            console.error(t('program_index.get_path_info_failed', { path: droppedPath }), error);
            ElMessage({ type: 'error', message: t('program_index.cannot_check_path', { path: droppedPath }) });
        }
    } // end for loop
};




// 用于控制对话框的状态
const dialogVisible = ref(false)
const editingProgram = ref<ProgramInfo | null>(null)
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

.lru-cache-hint {
    margin: -6px 0 12px 0;
    color: #909399;
    font-size: 12px;
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

/* 语义搜索内联说明块 */
.semantic-section { margin-bottom: 20px; }
.semantic-card { border: 1px dashed var(--el-border-color); background: var(--el-fill-color-lighter); }
.semantic-title { margin: 0 0 12px 0; font-size: 14px; line-height: 1.5; font-weight: 600; }
.semantic-tip + .semantic-tip { margin-top: 8px; }
.semantic-actions { margin-top: 16px; display: flex; flex-wrap: wrap; gap: 12px; }
</style>

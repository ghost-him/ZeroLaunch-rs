<template>
  <div class="settings-page">
    <h2 class="page-title">
      {{ t('program_index.extra_settings') }}
    </h2>
    <div class="content-container">
      <el-form
        label-width="auto"
        class="settings-form"
      >
        <el-form-item :label="t('program_index.change_search_algorithm')">
          <el-select
            v-model="config.program_manager_config.search_model"
            placeholder="standard"
            style="width: 240px"
            @change="(val: string) => configStore.updateConfig({ program_manager_config: { search_model: val } })"
          >
            <el-option
              v-for="item in search_model"
              :key="item.value"
              :label="item.label"
              :value="item.value"
            />
          </el-select>
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('program_index.search_algorithm_tooltip')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>

        <el-form-item :label="t('program_index.enable_lru_search_cache')">
          <el-switch
            v-model="config.program_manager_config.enable_lru_search_cache"
            @change="(val: boolean) =>
              configStore.updateConfig({
                program_manager_config: {
                  enable_lru_search_cache: val
                }
              })
            "
          />
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('program_index.lru_cache_tooltip')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>
        <p class="lru-cache-hint">
          {{ t('program_index.lru_cache_description') }}
        </p>

        <el-form-item
          v-if="config.program_manager_config.enable_lru_search_cache"
          :label="t('program_index.search_cache_capacity')"
        >
          <el-input-number
            v-model="config.program_manager_config.search_cache_capacity"
            :min="1"
            :max="1000"
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
        <div
          v-if="config.program_manager_config.search_model === 'semantic'"
          class="semantic-section"
        >
          <el-card
            shadow="never"
            class="semantic-card"
          >
            <p class="semantic-title">
              {{ t('program_index.semantic_search_intro') || '使用EmbeddingGemma-300m实现的语义搜索，带来无与论比的搜索体验。' }}
            </p>
            <el-alert
              type="info"
              :closable="false"
              show-icon
              class="semantic-tip"
            >
              <template #title>
                {{ t('program_index.semantic_tip_model') || '需要将模型单独下载到指定的目录中。' }}
              </template>
            </el-alert>
            <el-alert
              type="warning"
              :closable="false"
              show-icon
              class="semantic-tip"
            >
              <template #title>
                {{ t('program_index.semantic_tip_description') || '若补充完善描述信息，可让模型提供更准确的功能性搜索。' }}
              </template>
            </el-alert>
            <el-alert
              type="error"
              :closable="false"
              show-icon
              class="semantic-tip"
            >
              <template #title>
                {{ t('program_index.semantic_tip_performance') || '启用后在初始化与更新数据库时可能占用 GPU，并显著延长加载时间。' }}
              </template>
            </el-alert>
            <div class="semantic-actions">
              <el-button
                type="primary"
                @click="openModelFolder"
              >
                {{ t('program_index.open_model_folder') || '打开模型文件夹 (TODO)' }}
              </el-button>
              <el-button
                type="primary"
                @click="downloadModel"
              >
                {{ t('program_index.download_model') || '下载模型' }}
              </el-button>
            </div>
          </el-card>
        </div>

        <el-form-item :label="t('program_index.scan_uwp_apps')">
          <el-switch
            v-model="config.program_manager_config.loader.is_scan_uwp_programs"
            @change="(val: boolean) =>
              configStore.updateConfig({
                program_manager_config: {
                  loader: { is_scan_uwp_programs: val }
                }
              })
            "
          />
        </el-form-item>

        <!-- 排序算法参数设置 -->
        <el-divider />
        <h3>{{ t('program_index.sorting_algorithm_settings') }}</h3>
        <el-alert
          type="warning"
          :closable="false"
          show-icon
          style="margin-bottom: 20px;"
        >
          <template #title>
            {{ t('program_index.sorting_params_warning') }}
          </template>
        </el-alert>

        <el-form-item :label="t('program_index.enable_sorting')">
          <el-switch
            v-model="config.program_manager_config.ranker.is_enable"
            @change="(val: boolean) =>
              configStore.updateConfig({
                program_manager_config: {
                  ranker: { is_enable: val }
                }
              })
            "
          />
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('program_index.enable_sorting_tooltip')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>

        <div v-if="config.program_manager_config.ranker.is_enable">
          <el-form-item :label="t('program_index.history_weight')">
            <el-input-number
              v-model="config.program_manager_config.ranker.history_weight"
              :min="0"
              :max="10"
              :step="0.1"
              :precision="2"
              @change="(val: number) =>
                configStore.updateConfig({
                  program_manager_config: {
                    ranker: { history_weight: val ?? 0.8 }
                  }
                })
              "
            />
            <el-tooltip
              class="box-item"
              effect="dark"
              placement="right"
              style="max-width: 400px;"
            >
              <template #content>
                <div style="max-width: 400px;">
                  {{ t('program_index.history_weight_description') }}
                </div>
              </template>
              <el-icon class="el-question-icon">
                <QuestionFilled />
              </el-icon>
            </el-tooltip>
          </el-form-item>

          <el-form-item :label="t('program_index.recent_habit_weight')">
            <el-input-number
              v-model="config.program_manager_config.ranker.recent_habit_weight"
              :min="0"
              :max="10"
              :step="0.1"
              :precision="2"
              @change="(val: number) =>
                configStore.updateConfig({
                  program_manager_config: {
                    ranker: { recent_habit_weight: val ?? 1.5 }
                  }
                })
              "
            />
            <el-tooltip
              class="box-item"
              effect="dark"
              placement="right"
            >
              <template #content>
                <div style="max-width: 400px;">
                  {{ t('program_index.recent_habit_weight_description') }}
                </div>
              </template>
              <el-icon class="el-question-icon">
                <QuestionFilled />
              </el-icon>
            </el-tooltip>
          </el-form-item>

          <el-form-item :label="t('program_index.temporal_weight')">
            <el-input-number
              v-model="config.program_manager_config.ranker.temporal_weight"
              :min="0"
              :max="10"
              :step="0.1"
              :precision="2"
              @change="(val: number) =>
                configStore.updateConfig({
                  program_manager_config: {
                    ranker: { temporal_weight: val ?? 0.5 }
                  }
                })
              "
            />
            <el-tooltip
              class="box-item"
              effect="dark"
              placement="right"
            >
              <template #content>
                <div style="max-width: 400px;">
                  {{ t('program_index.temporal_weight_description') }}
                </div>
              </template>
              <el-icon class="el-question-icon">
                <QuestionFilled />
              </el-icon>
            </el-tooltip>
          </el-form-item>

          <el-form-item :label="t('program_index.query_affinity_weight')">
            <el-input-number
              v-model="config.program_manager_config.ranker.query_affinity_weight"
              :min="0"
              :max="10"
              :step="0.1"
              :precision="2"
              @change="(val: number) =>
                configStore.updateConfig({
                  program_manager_config: {
                    ranker: { query_affinity_weight: val ?? 5.0 }
                  }
                })
              "
            />
            <el-tooltip
              class="box-item"
              effect="dark"
              placement="right"
            >
              <template #content>
                <div style="max-width: 400px;">
                  {{ t('program_index.query_affinity_weight_description') }}
                </div>
              </template>
              <el-icon class="el-question-icon">
                <QuestionFilled />
              </el-icon>
            </el-tooltip>
          </el-form-item>

          <el-form-item :label="t('program_index.query_affinity_time_decay')">
            <el-input-number
              v-model="config.program_manager_config.ranker.query_affinity_time_decay"
              :min="0"
              :max="1000000"
              :step="3600"
              @change="(val: number) =>
                configStore.updateConfig({
                  program_manager_config: {
                    ranker: { query_affinity_time_decay: val ?? 259200 }
                  }
                })
              "
            />
            <el-tooltip
              class="box-item"
              effect="dark"
              placement="right"
            >
              <template #content>
                <div style="max-width: 400px;">
                  {{ t('program_index.query_affinity_time_decay_description') }}
                </div>
              </template>
              <el-icon class="el-question-icon">
                <QuestionFilled />
              </el-icon>
            </el-tooltip>
          </el-form-item>

          <el-form-item :label="t('program_index.query_affinity_cooldown')">
            <el-input-number
              v-model="config.program_manager_config.ranker.query_affinity_cooldown"
              :min="0"
              :max="3600"
              :step="10"
              @change="(val: number) =>
                configStore.updateConfig({
                  program_manager_config: {
                    ranker: { query_affinity_cooldown: val ?? 60 }
                  }
                })
              "
            />
            <el-tooltip
              class="box-item"
              effect="dark"
              placement="right"
            >
              <template #content>
                <div style="max-width: 400px;">
                  {{ t('program_index.query_affinity_cooldown_description') }}
                </div>
              </template>
              <el-icon class="el-question-icon">
                <QuestionFilled />
              </el-icon>
            </el-tooltip>
          </el-form-item>

          <el-form-item :label="t('program_index.temporal_decay')">
            <el-input-number
              v-model="config.program_manager_config.ranker.temporal_decay"
              :min="0"
              :max="1000000"
              :step="3600"
              @change="(val: number) =>
                configStore.updateConfig({
                  program_manager_config: {
                    ranker: { temporal_decay: val ?? 10800 }
                  }
                })
              "
            />
            <el-tooltip
              class="box-item"
              effect="dark"
              placement="right"
            >
              <template #content>
                <div style="max-width: 400px;">
                  {{ t('program_index.temporal_decay_description') }}
                </div>
              </template>
              <el-icon class="el-question-icon">
                <QuestionFilled />
              </el-icon>
            </el-tooltip>
          </el-form-item>
        </div>
      </el-form>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRemoteConfigStore } from '../../../stores/remote_config'
import { storeToRefs } from 'pinia'
import { QuestionFilled } from '@element-plus/icons-vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

const { t } = useI18n()
const configStore = useRemoteConfigStore()
const { config } = storeToRefs(configStore)

const search_model = computed(() => [
    { value: 'standard', label: t('program_index.standard_search_algorithm') },
    { value: 'skim', label: t('program_index.skim_matching_algorithm') },
    { value: 'launchy', label: t('program_index.launchyqt_algorithm') },
    { value: 'semantic', label: t('program_index.semantic_search_algorithm') },
])

const openModelFolder = async () => {
    await invoke('command_open_models_dir')
}

const downloadModel = async () => {
    const selected = await open({
        directory: true,
        multiple: false,
        title: t('program_index.select_download_folder') || '选择下载保存的文件夹',
    });
    
    if (selected) {
        await invoke('command_download_model', { savePath: selected });
    }
}
</script>

<style scoped>
.settings-page {
    padding: 20px;
    height: 100%;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
}

.content-container {
    flex: 1;
    overflow-y: auto;
}

.page-title {
    margin-top: 0;
    margin-bottom: 20px;
    font-size: 20px;
    font-weight: 500;
    color: #303133;
}

.settings-form {
    max-width: 800px;
}

.lru-cache-hint {
    margin: -6px 0 12px 0;
    color: #909399;
    font-size: 12px;
}

.el-question-icon {
    margin-left: 8px;
    color: #909399;
    cursor: help;
}

.semantic-section { margin-bottom: 20px; }
.semantic-card { border: 1px dashed var(--el-border-color); background: var(--el-fill-color-lighter); }
.semantic-title { margin: 0 0 12px 0; font-size: 14px; line-height: 1.5; font-weight: 600; }
.semantic-tip + .semantic-tip { margin-top: 8px; }
.semantic-actions { margin-top: 16px; display: flex; flex-wrap: wrap; gap: 12px; }
</style>

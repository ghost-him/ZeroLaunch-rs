<template>
  <div class="settings-page">
    <h2 class="page-title">
      {{ t('everything.everything_search_settings') }}
    </h2>
    <div class="content-container">
      <el-form
        label-width="auto"
        class="settings-form"
      >
        <el-form-item :label="t('everything.sort_method')">
          <el-select
            v-model="everything_config.sort_method"
            style="width: 240px"
            @change="updateSortMethod"
          >
            <el-option
              v-for="item in sortMethods"
              :key="item.value"
              :label="item.label"
              :value="item.value"
            />
          </el-select>
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('everything.sort_method_tooltip')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>

        <el-form-item :label="t('everything.sort_threshold')">
          <el-input-number
            v-model="everything_config.sort_threshold"
            :min="1"
            :step="1"
            :precision="0"
            @change="updateSortThreshold"
          />
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('everything.sort_threshold_tooltip')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>

        <el-form-item :label="t('everything.result_limit')">
          <el-input-number
            v-model="everything_config.result_limit"
            :min="1"
            :step="1"
            :precision="0"
            @change="updateResultLimit"
          />
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('everything.result_limit_tooltip')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>
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
import type { EverythingSortKind } from '../../../api/remote_config_types'

const { t } = useI18n()
const configStore = useRemoteConfigStore()
const { config } = storeToRefs(configStore)

const everything_config = computed({
    get: () => config.value.everything_config,
    set: (value) => {
        configStore.updateConfig({
            everything_config: value,
        })
    },
})

const sortMethods = computed(() => [
    { label: t('everything.sort_name_ascending'), value: 'NameAscending' },
    { label: t('everything.sort_name_descending'), value: 'NameDescending' },
    { label: t('everything.sort_path_ascending'), value: 'PathAscending' },
    { label: t('everything.sort_path_descending'), value: 'PathDescending' },
    { label: t('everything.sort_size_ascending'), value: 'SizeAscending' },
    { label: t('everything.sort_size_descending'), value: 'SizeDescending' },
    { label: t('everything.sort_extension_ascending'), value: 'ExtensionAscending' },
    { label: t('everything.sort_extension_descending'), value: 'ExtensionDescending' },
    { label: t('everything.sort_type_name_ascending'), value: 'TypeNameAscending' },
    { label: t('everything.sort_type_name_descending'), value: 'TypeNameDescending' },
    { label: t('everything.sort_date_created_ascending'), value: 'DateCreatedAscending' },
    { label: t('everything.sort_date_created_descending'), value: 'DateCreatedDescending' },
    { label: t('everything.sort_date_modified_ascending'), value: 'DateModifiedAscending' },
    { label: t('everything.sort_date_modified_descending'), value: 'DateModifiedDescending' },
    { label: t('everything.sort_attributes_ascending'), value: 'AttributesAscending' },
    { label: t('everything.sort_attributes_descending'), value: 'AttributesDescending' },
    { label: t('everything.sort_file_list_filename_ascending'), value: 'FileListFilenameAscending' },
    { label: t('everything.sort_file_list_filename_descending'), value: 'FileListFilenameDescending' },
    { label: t('everything.sort_run_count_ascending'), value: 'RunCountAscending' },
    { label: t('everything.sort_run_count_descending'), value: 'RunCountDescending' },
    { label: t('everything.sort_date_recently_changed_ascending'), value: 'DateRecentlyChangedAscending' },
    { label: t('everything.sort_date_recently_changed_descending'), value: 'DateRecentlyChangedDescending' },
    { label: t('everything.sort_date_accessed_ascending'), value: 'DateAccessedAscending' },
    { label: t('everything.sort_date_accessed_descending'), value: 'DateAccessedDescending' },
    { label: t('everything.sort_date_run_ascending'), value: 'DateRunAscending' },
    { label: t('everything.sort_date_run_descending'), value: 'DateRunDescending' },
])

const updateSortMethod = (value: EverythingSortKind) => {
    configStore.updateConfig({
        everything_config: {
            sort_method: value,
        },
    })
}

const updateSortThreshold = (value: number | undefined) => {
    if (value !== undefined && value >= 1) {
        configStore.updateConfig({
            everything_config: {
                sort_threshold: Math.floor(value),
            },
        })
    }
}

const updateResultLimit = (value: number | undefined) => {
    if (value !== undefined && value >= 1) {
        configStore.updateConfig({
            everything_config: {
                result_limit: Math.floor(value),
            },
        })
    }
}
</script>

<style scoped>
.settings-page {
  padding: 20px;
}

.page-title {
  font-size: 24px;
  font-weight: bold;
  margin-bottom: 20px;
}

.content-container {
  background: white;
  padding: 20px;
  border-radius: 8px;
}

.settings-form {
  max-width: 800px;
}

.el-question-icon {
  margin-left: 8px;
  cursor: help;
  color: #909399;
}
</style>

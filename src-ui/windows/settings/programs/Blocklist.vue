<template>
  <div class="settings-page">
    <h2 class="page-title">
      {{ t('program_index.set_blocked_paths') }}
    </h2>
    <div class="content-container">
      <!-- Browse and Block Part (Similar to Aliases) -->
      <section class="section">
        <h3 class="section-title">
          {{ t('program_index.browse_programs') }}
        </h3>
        <div class="search-bar-row">
          <el-input
            v-model="searchKeyword"
            :placeholder="t('icon_management.search_placeholder')"
            :prefix-icon="Search"
            clearable
            :disabled="showAllMode"
            @input="handleSearch"
            class="search-input"
          />
          <el-button
            :type="showAllMode ? 'primary' : 'default'"
            @click="toggleShowAll"
          >
            {{ showAllMode ? t('icon_management.back_to_search') : t('icon_management.show_all') }}
          </el-button>
        </div>

        <div class="table-wrapper program-table">
          <el-table
            v-loading="loading"
            :data="programList"
            stripe
            style="width: 100%"
            height="300px"
          >
            <el-table-column
              :label="t('icon_management.icon')"
              width="60"
            >
              <template #default="scope">
                <img
                  :src="getIconUrl(scope.row.icon_request_json)"
                  class="program-icon"
                  alt="icon"
                >
              </template>
            </el-table-column>

            <el-table-column
              :label="t('icon_management.program_name')"
              prop="name"
              width="150"
              show-overflow-tooltip
            />

            <el-table-column
              :label="t('icon_management.path')"
              prop="path"
              show-overflow-tooltip
            />

            <el-table-column
              :label="t('program_index.operation')"
              width="100"
              fixed="right"
            >
              <template #default="{ row }">
                <el-button
                  size="small"
                  type="warning"
                  plain
                  @click="handleBlockProgram(row.path)"
                >
                  {{ t('program_index.block_program') }}
                </el-button>
              </template>
            </el-table-column>
          </el-table>
        </div>
      </section>

      <!-- Current Blocked List Part -->
      <section class="section blocked-section">
        <h3 class="section-title">
          {{ t('program_index.blocked_list') }}
        </h3>
        <div class="action-buttons">
          <el-button
            type="primary"
            @click="handleSelectFolder"
          >
            <el-icon><Folder /></el-icon> {{ t('program_index.add_folder') }}
          </el-button>
          <el-button
            type="primary"
            @click="handleSelectFile"
          >
            <el-icon><Document /></el-icon> {{ t('program_index.add_file') }}
          </el-button>
          <el-button @click="addForbiddenPath">
            <el-icon><Plus /></el-icon> {{ t('program_index.add_item') }}
          </el-button>
        </div>

        <div class="table-wrapper">
          <el-table
            :data="forbidden_paths_data"
            stripe
            style="width: 100%; margin-top: 10px;"
          >
            <el-table-column :label="t('program_index.target_blocked_path')">
              <template #default="{ $index }">
                <el-input
                  v-model="forbidden_paths[$index]"
                  size="small"
                  :placeholder="t('program_index.enter_target_path')"
                  @change="updateForbiddenPaths"
                />
              </template>
            </el-table-column>
            <el-table-column
              fixed="right"
              :label="t('program_index.operation')"
              width="100"
            >
              <template #default="{ $index }">
                <el-button
                  link
                  size="small"
                  type="danger"
                  @click="deleteForbiddenPath($index)"
                >
                  {{ t('program_index.delete_row') }}
                </el-button>
              </template>
            </el-table-column>
          </el-table>
        </div>
      </section>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRemoteConfigStore } from '../../../stores/remote_config'
import { storeToRefs } from 'pinia'
import { Plus, Search, Folder, Document } from '@element-plus/icons-vue'
import { useProgramSearch } from '../../../composables/useProgramSearch'
import { open } from '@tauri-apps/plugin-dialog'
import { homeDir } from '@tauri-apps/api/path'
import { ElMessage } from 'element-plus'

const { t } = useI18n()
const configStore = useRemoteConfigStore()
const { config } = storeToRefs(configStore)

const {
    searchKeyword,
    loading,
    programList,
    showAllMode,
    handleSearch,
    toggleShowAll,
    getIconUrl
} = useProgramSearch()

const forbidden_paths = computed({
    get: () => config.value.program_manager_config.loader.forbidden_paths,
    set: (value) => {
        configStore.updateConfig({
            program_manager_config: {
                loader: { forbidden_paths: value },
            },
        })
    },
})

const forbidden_paths_data = computed(() => {
    return forbidden_paths.value.map(p => ({ path: p }))
})

const updateForbiddenPaths = () => {
    configStore.updateConfig({
        program_manager_config: {
            loader: { forbidden_paths: forbidden_paths.value },
        },
    })
}

const deleteForbiddenPath = (index: number) => {
    const newPaths = [...forbidden_paths.value]
    newPaths.splice(index, 1)
    forbidden_paths.value = newPaths
}

const addForbiddenPath = () => {
    forbidden_paths.value = [...forbidden_paths.value, '']
}

const handleBlockProgram = (path: string) => {
    if (forbidden_paths.value.includes(path)) {
        ElMessage.warning(t('program_index.path_already_exists', { path }))
        return
    }
    forbidden_paths.value = [...forbidden_paths.value, path]
    ElMessage.success(t('app.block_success_title'))
}

const handleSelectFolder = async () => {
    const folderSelected = await open({
        directory: true,
        multiple: false,
        defaultPath: await homeDir()
    })
    if (folderSelected) {
        const picked = Array.isArray(folderSelected) ? folderSelected[0] : folderSelected as string
        if (forbidden_paths.value.includes(picked)) {
            ElMessage.warning(t('program_index.path_already_exists', { path: picked }))
            return
        }
        forbidden_paths.value = [...forbidden_paths.value, picked]
    }
}

const handleSelectFile = async () => {
    const fileSelected = await open({
        directory: false,
        multiple: false,
        defaultPath: await homeDir()
    })
    if (fileSelected) {
        const picked = Array.isArray(fileSelected) ? fileSelected[0] : fileSelected as string
        if (forbidden_paths.value.includes(picked)) {
            ElMessage.warning(t('program_index.path_already_exists', { path: picked }))
            return
        }
        forbidden_paths.value = [...forbidden_paths.value, picked]
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
    overflow-y: hidden;
}

.page-title {
    margin-top: 0;
    margin-bottom: 20px;
    font-size: 20px;
    font-weight: 500;
    color: #303133;
}

.content-container {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    gap: 30px;
    padding-right: 10px;
}

.section {
    display: flex;
    flex-direction: column;
    gap: 15px;
}

.section-title {
    margin: 0;
    font-size: 16px;
    font-weight: 500;
    color: #606266;
}

.search-bar-row {
  display: flex;
  gap: 10px;
  margin-bottom: 5px;
}

.search-input {
  flex: 1;
}

.table-wrapper {
    border: 1px solid #ebeef5;
    border-radius: 4px;
}

.program-table {
    max-height: 400px;
}

.program-icon {
    width: 32px;
    height: 32px;
    object-fit: contain;
}

.action-buttons {
    display: flex;
    gap: 10px;
}

.blocked-section {
    margin-bottom: 20px;
}
</style>

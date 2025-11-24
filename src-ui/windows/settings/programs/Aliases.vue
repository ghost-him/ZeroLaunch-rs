<template>
  <div class="settings-page">
    <h2 class="page-title">
      {{ t('program_index.setting_alias') }}
    </h2>
    <div class="content-container">
      <el-input
        v-model="searchKeyword"
        :placeholder="t('icon_management.search_placeholder')"
        :prefix-icon="Search"
        clearable
        @input="handleSearch"
        style="margin-bottom: 16px;"
      />

      <div class="table-wrapper">
        <el-table
          v-loading="loading"
          :data="programList"
          style="width: 100%"
          height="100%"
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
            width="200"
          />

          <el-table-column
            :label="t('icon_management.path')"
            prop="path"
            show-overflow-tooltip
          />

          <el-table-column :label="t('program_index.setting_alias')">
            <template #default="{ row }">
              <div class="alias-tags">
                <el-tag
                  v-for="alias in getAliases(row.path)"
                  :key="alias"
                  size="small"
                  class="alias-tag"
                >
                  {{ alias }}
                </el-tag>
              </div>
            </template>
          </el-table-column>

          <el-table-column
            :label="t('program_index.operation')"
            width="120"
            fixed="right"
          >
            <template #default="{ row }">
              <el-button
                size="small"
                type="primary"
                @click="openEditDialog(row)"
              >
                {{ t('program_index.setting_alias') }}
              </el-button>
            </template>
          </el-table-column>
        </el-table>
      </div>
    </div>

    <el-dialog
      v-if="editingProgram"
      v-model="dialogVisible"
      :title="t('settings.edit_program_alias', { name: editingProgram.name })"
      width="500"
    >
      <div style="display: flex; flex-direction: column; gap: 10px;">
        <div
          v-for="(alias, index) in getAliases(editingProgram.path)"
          :key="index"
          style="display: flex; align-items: center; gap: 10px;"
        >
          <el-input
            :model-value="alias"
            :placeholder="t('settings.enter_alias')"
            @update:model-value="(newValue: string) => updateAliasInDialog(index, newValue)"
          />
          <el-button
            type="danger"
            @click="removeAliasInDialog(index)"
          >
            {{ t('settings.delete') }}
          </el-button>
        </div>
      </div>
      <template #footer>
        <div class="dialog-footer">
          <el-button
            style="width: 100%; margin-bottom: 10px;"
            @click="addAliasInDialog"
          >
            {{
              t('settings.add_alias') }}
          </el-button>
          <el-button
            type="primary"
            @click="dialogVisible = false"
          >
            {{ t('settings.close') }}
          </el-button>
        </div>
      </template>
    </el-dialog>
  </div>
</template>

<script lang="ts" setup>
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRemoteConfigStore } from '../../../stores/remote_config'
import { storeToRefs } from 'pinia'
import { ElButton, ElTag, ElInput, ElTable, ElTableColumn, ElDialog } from 'element-plus'
import { Search } from '@element-plus/icons-vue'
import type { ProgramDisplayInfo } from '../../../api/program'
import { useProgramSearch } from '../../../composables/useProgramSearch'

const { t } = useI18n()
const configStore = useRemoteConfigStore()
const { config } = storeToRefs(configStore)

const {
    searchKeyword,
    loading,
    programList,
    handleSearch,
    getIconUrl
} = useProgramSearch()

const dialogVisible = ref(false)
const editingProgram = ref<ProgramDisplayInfo | null>(null)

const program_alias = computed({
    get: () => config.value.program_manager_config.loader.program_alias,
    set: (value) => {
        configStore.updateConfig({
            program_manager_config: {
                loader: { program_alias: value },
            },
        })
    },
})

const getAliases = (path: string) => {
    return program_alias.value[path] || []
}

const openEditDialog = (row: ProgramDisplayInfo) => {
    editingProgram.value = row
    // Ensure entry exists in map
    if (!program_alias.value[row.path]) {
        const newAliasMap = { ...program_alias.value }
        newAliasMap[row.path] = []
        program_alias.value = newAliasMap
    }
    dialogVisible.value = true
}

const updateAliasInDialog = (index: number, newValue: string) => {
    if (!editingProgram.value) return
    const path = editingProgram.value.path
    const aliases = [...(program_alias.value[path] || [])]
    aliases[index] = newValue
    
    const newAliasMap = { ...program_alias.value }
    newAliasMap[path] = aliases
    program_alias.value = newAliasMap
}

const removeAliasInDialog = (index: number) => {
    if (!editingProgram.value) return
    const path = editingProgram.value.path
    const aliases = [...(program_alias.value[path] || [])]
    aliases.splice(index, 1)
    
    const newAliasMap = { ...program_alias.value }
    // If empty, maybe we should keep the key or delete it? 
    // Keeping it is safer for now, or we can clean up empty entries.
    // Let's keep it simple.
    newAliasMap[path] = aliases
    program_alias.value = newAliasMap
}

const addAliasInDialog = () => {
    if (!editingProgram.value) return
    const path = editingProgram.value.path
    const aliases = [...(program_alias.value[path] || [])]
    aliases.push('')
    
    const newAliasMap = { ...program_alias.value }
    newAliasMap[path] = aliases
    program_alias.value = newAliasMap
}

// Initial load
handleSearch()
</script>

<style scoped>
.settings-page {
    padding: 20px;
    height: 100%;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
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
    overflow-x: hidden;
}

.table-wrapper {
    flex: 1;
    min-height: 0;
}

.program-icon {
    width: 32px;
    height: 32px;
    object-fit: contain;
}

.alias-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
}

.alias-tag {
    margin-right: 4px;
}
</style>

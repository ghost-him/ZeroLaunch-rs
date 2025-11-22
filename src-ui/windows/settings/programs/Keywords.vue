<template>
  <div class="settings-page">
    <h2 class="page-title">
      {{ t('program_index.set_fixed_offset') }}
    </h2>
    <div class="content-container">
      <el-button
        class="add-btn"
        @click="addKeyFilter"
      >
        <el-icon><Plus /></el-icon> {{ t('program_index.add_item') }}
      </el-button>
      <el-table
        :data="keyFilterData"
        stripe
        style="width: 100%; margin-top: 10px;"
      >
        <el-table-column :label="t('program_index.target_keyword')">
          <template #default="{ row }">
            <el-input
              v-model="row.key"
              size="small"
              :placeholder="t('program_index.enter_target_keyword')"
              @change="updateProgramBias(row)"
            />
          </template>
        </el-table-column>
        <el-table-column
          :label="t('program_index.offset')"
          show-overflow-tooltip
        >
          <template #default="{ row }">
            <el-input-number
              v-model="row.bias"
              size="small"
              :placeholder="t('program_index.enter_offset')"
              @change="updateProgramBias(row)"
            />
          </template>
        </el-table-column>
        <el-table-column
          :label="t('program_index.note')"
          show-overflow-tooltip
        >
          <template #default="{ row }">
            <el-input
              v-model="row.note"
              size="small"
              :placeholder="t('program_index.enter_note')"
              @change="updateProgramBias(row)"
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
              @click="deleteKeyFilterRow($index)"
            >
              {{ t('program_index.delete_row') }}
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRemoteConfigStore } from '../../../stores/remote_config'
import { storeToRefs } from 'pinia'
import { Plus } from '@element-plus/icons-vue'

const { t } = useI18n()
const configStore = useRemoteConfigStore()
const { config } = storeToRefs(configStore)

interface KeyFilterData {
    originalKey: string
    key: string
    bias: number
    note: string
}

const keyFilterData = computed(() => {
    const bias = config.value.program_manager_config.loader.program_bias
    return Object.keys(bias).map(key => ({
        originalKey: key,
        key,
        bias: bias[key][0],
        note: bias[key][1] || '',
    }))
})

const updateProgramBias = (row: KeyFilterData) => {
    const newProgramBias = { ...config.value.program_manager_config.loader.program_bias }
    if (row.originalKey !== row.key) {
        delete newProgramBias[row.originalKey]
    }
    newProgramBias[row.key] = [row.bias, row.note]
    configStore.updateConfig({
        program_manager_config: {
            loader: { program_bias: newProgramBias },
        },
    })
}

const deleteKeyFilterRow = (index: number) => {
    const newProgramBias = JSON.parse(JSON.stringify(config.value.program_manager_config.loader.program_bias))
    const keyToDelete = keyFilterData.value[index].key
    delete newProgramBias[keyToDelete]
    configStore.updateConfig({
        program_manager_config: {
            loader: { program_bias: newProgramBias },
        },
    })
}

const addKeyFilter = () => {
    const newProgramBias = { ...config.value.program_manager_config.loader.program_bias }
    const newKey = t('program_index.enter_keyword_placeholder')
    newProgramBias[newKey] = [0, '']
    configStore.updateConfig({
        program_manager_config: {
            loader: { program_bias: newProgramBias },
        },
    })
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
}

.add-btn {
    width: 100%;
    flex-shrink: 0;
}
</style>

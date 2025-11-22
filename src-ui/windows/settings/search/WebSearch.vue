<template>
  <div class="settings-page">
    <h2 class="page-title">
      {{ t('settings.custom_web_search') }}
    </h2>
    <div class="content-container">
      <el-button
        class="add-btn"
        @click="addIndexWebPage"
      >
        <el-icon><Plus /></el-icon> {{ t('settings.add_item') }}
      </el-button>
      <el-table
        :data="index_web_pages"
        stripe
        style="width: 100%; margin-top: 10px;"
      >
        <el-table-column
          :label="t('settings.keyword_for_search')"
          show-overflow-tooltip
          fixed="left"
          width="150"
        >
          <template #default="scope">
            <el-input
              v-model="index_web_pages[scope.$index][0]"
              size="small"
              :placeholder="t('settings.enter_keyword')"
              @change="updateIndexWebPages"
            />
          </template>
        </el-table-column>
        <el-table-column
          :label="t('settings.target_website_address')"
          show-overflow-tooltip
        >
          <template #default="scope">
            <div style="display: flex; gap: 5px;">
              <el-select
                :model-value="getProtocol(scope.$index)"
                size="small"
                style="width: 90px;"
                @update:model-value="(value: string) => updateProtocol(scope.$index, value)"
              >
                <el-option
                  label="https://"
                  value="https://"
                />
                <el-option
                  label="http://"
                  value="http://"
                />
              </el-select>
              <el-input
                :model-value="getUrlWithoutProtocol(scope.$index)"
                size="small"
                :placeholder="t('settings.enter_target_path_without_protocol')"
                style="flex: 1;"
                @update:model-value="(value: string) => updateUrlWithoutProtocol(scope.$index, value)"
              />
            </div>
          </template>
        </el-table-column>
        <el-table-column
          fixed="right"
          :label="t('settings.actions')"
          width="100"
        >
          <template #default="scope">
            <el-button
              link
              size="small"
              type="danger"
              @click="deleteIndexWebPages(scope.$index)"
            >
              {{ t('settings.delete_row') }}
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

const index_web_pages = computed({
    get: () => config.value.program_manager_config.loader.index_web_pages,
    set: (value) => {
        configStore.updateConfig({
            program_manager_config: {
                loader: { index_web_pages: value },
            },
        })
    },
})

const deleteIndexWebPages = (index: number) => {
    index_web_pages.value = index_web_pages.value.filter((_, i) => i !== index)
}

const updateIndexWebPages = () => {
    configStore.updateConfig({
        program_manager_config: {
            loader: { index_web_pages: index_web_pages.value },
        },
    })
}

const addIndexWebPage = () => {
    index_web_pages.value = [...index_web_pages.value, ['', 'https://']]
}

const getProtocol = (index: number): string => {
    const url = index_web_pages.value[index]?.[1] || ''
    if (url.startsWith('https://')) {
        return 'https://'
    } else if (url.startsWith('http://')) {
        return 'http://'
    }
    return 'https://'
}

const getUrlWithoutProtocol = (index: number): string => {
    const url = index_web_pages.value[index]?.[1] || ''
    if (url.startsWith('https://')) {
        return url.substring(8)
    } else if (url.startsWith('http://')) {
        return url.substring(7)
    }
    return url
}

const updateProtocol = (index: number, protocol: string) => {
    const urlWithoutProtocol = getUrlWithoutProtocol(index)
    index_web_pages.value[index][1] = protocol + urlWithoutProtocol
    updateIndexWebPages()
}

const updateUrlWithoutProtocol = (index: number, urlWithoutProtocol: string) => {
    const protocol = getProtocol(index)
    index_web_pages.value[index][1] = protocol + urlWithoutProtocol
    updateIndexWebPages()
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
    overflow: hidden;
}

.add-btn {
    width: 100%;
    flex-shrink: 0;
}
</style>

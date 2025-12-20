<template>
  <div class="settings-page">
    <h2 class="page-title">
      {{ t('settings.custom_web_search') }}
    </h2>
    <div class="content-container">
      <div class="button-group">
        <el-button
          class="add-btn"
          @click="addIndexWebPage"
        >
          <el-icon><Plus /></el-icon> {{ t('settings.add_item') }}
        </el-button>
        <el-button
          class="import-btn"
          @click="showImportDialog = true"
        >
          <el-icon><Download /></el-icon> {{ t('settings.import_from_browser') }}
        </el-button>
      </div>
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
            <el-input
              v-model="index_web_pages[scope.$index][1]"
              size="small"
              :placeholder="t('settings.enter_target_path')"
              @change="updateIndexWebPages"
            />
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

    <BrowserImportDialog
      v-model="showImportDialog"
      @import="handleImportBookmarks"
    />
  </div>
</template>

<script lang="ts" setup>
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRemoteConfigStore } from '../../../stores/remote_config'
import { storeToRefs } from 'pinia'
import { Plus, Download } from '@element-plus/icons-vue'
import BrowserImportDialog from './BrowserImportDialog.vue'

const { t } = useI18n()
const configStore = useRemoteConfigStore()
const { config } = storeToRefs(configStore)
const showImportDialog = ref(false)

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
    index_web_pages.value = [...index_web_pages.value, ['', '']]
}

const handleImportBookmarks = (bookmarks: Array<{ title: string, url: string }>) => {
    const newPages = bookmarks.map(b => [b.title, b.url] as [string, string])
    // Append new bookmarks to existing ones
    index_web_pages.value = [...index_web_pages.value, ...newPages]
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

.button-group {
    display: flex;
    gap: 10px;
    width: 100%;
    flex-shrink: 0;
}

.add-btn, .import-btn {
    flex: 1;
}
</style>

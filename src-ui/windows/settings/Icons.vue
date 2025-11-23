<template>
  <div class="settings-page">
    <h2 class="page-title">
      {{ t('settings.icon_management') }}
    </h2>
    <div class="content-container">
      <el-form
        label-width="auto"
        class="settings-form"
      >
        <el-form-item :label="t('icon_management.enable_online_icon_loading')">
          <el-switch
            v-model="config.icon_manager_config.enable_online"
            @change="(val: boolean) =>
              configStore.updateConfig({
                icon_manager_config: { enable_online: val }
              })
            "
          />
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('icon_management.online_icon_tooltip')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>

        <el-form-item :label="t('icon_management.enable_icon_cache')">
          <el-switch
            v-model="config.icon_manager_config.enable_icon_cache"
            @change="(val: boolean) =>
              configStore.updateConfig({
                icon_manager_config: { enable_icon_cache: val }
              })
            "
          />
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('icon_management.icon_cache_tooltip')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>
        
        <el-form-item>
            <el-button
            type="primary"
            @click="openIconCacheDir"
            >
            {{ t('icon_management.open_icon_cache_folder') }}
            </el-button>
        </el-form-item>
      </el-form>
    </div>
  </div>
</template>

<script setup lang="ts">
import { storeToRefs } from 'pinia'
import { useRemoteConfigStore } from '../../stores/remote_config'
import { useI18n } from 'vue-i18n'
import { QuestionFilled } from '@element-plus/icons-vue'
import { invoke } from '@tauri-apps/api/core'

const { t } = useI18n()
const configStore = useRemoteConfigStore()
const { config } = storeToRefs(configStore)

const openIconCacheDir = async () => {
  try {
    await invoke('command_open_icon_cache_dir')
  } catch (e) {
    console.error('Failed to open icon cache dir', e)
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

.el-question-icon {
    margin-left: 8px;
    color: #909399;
    cursor: help;
}
</style>

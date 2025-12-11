<template>
  <div class="settings-page">
    <h2 class="page-title">
      {{ t('settings.menu.general') }}
    </h2>
    <div class="content-container">
      <el-form label-width="auto">
        <el-divider content-position="left">
          {{ t('app_config.language_settings') }}
        </el-divider>
        <el-form-item :label="t('app_config.language')">
          <el-select
            v-model="currentLanguage"
            style="width: 200px;"
            @change="changeLanguage"
          >
            <el-option
              :label="t('app_config.chinese')"
              value="zh-Hans"
            />
            <el-option
              :label="t('app_config.chinese_traditional')"
              value="zh-Hant"
            />
            <el-option
              :label="t('app_config.english')"
              value="en"
            />
          </el-select>
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('app_config.language_save_tip')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>

        <el-divider content-position="left">
          {{ t('app_config.startup_instance') }}
        </el-divider>
        <el-form-item :label="t('app_config.auto_start')">
          <el-switch
            v-model="config.app_config.is_auto_start"
            @change="(val: boolean) => configStore.updateConfig({ app_config: { is_auto_start: val } })"
          />
        </el-form-item>

        <el-form-item :label="t('app_config.silent_start')">
          <el-switch
            v-model="config.app_config.is_silent_start"
            @change="(val: boolean) => configStore.updateConfig({ app_config: { is_silent_start: val } })"
          />
        </el-form-item>

        <el-form-item :label="t('app_config.launch_new_on_failure')">
          <el-switch
            v-model="config.app_config.launch_new_on_failure"
            @change="(val: boolean) => configStore.updateConfig({ app_config: { launch_new_on_failure: val } })"
          />
        </el-form-item>

        <el-divider content-position="left">
          {{ t('app_config.search_data') }}
        </el-divider>
        <el-form-item :label="t('app_config.search_result_count')">
          <el-input-number
            v-model="config.app_config.search_result_count"
            :step="1"
            :precision="0"
            :min="1"
            @change="(val: number) => configStore.updateConfig({ app_config: { search_result_count: val } })"
          />
        </el-form-item>

        <el-form-item :label="t('app_config.scroll_threshold')">
          <el-input-number
            v-model="config.app_config.scroll_threshold"
            :step="1"
            :precision="0"
            :min="1"
            @change="(val: number) => configStore.updateConfig({ app_config: { scroll_threshold: val } })"
          />
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('app_config.scroll_threshold_tooltip')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>

        <el-form-item :label="t('refresh_scheduler.auto_refresh_time')">
          <el-input-number
            v-model="config.refresh_scheduler_config.auto_refresh_interval_mins"
            :step="1"
            :precision="0"
            :min="1"
            @change="(val: number) => configStore.updateConfig({ refresh_scheduler_config: { auto_refresh_interval_mins: val } })"
          >
            <template #suffix>
              <span>{{ t('app_config.minutes') }}</span>
            </template>
          </el-input-number>
        </el-form-item>

        <el-form-item :label="t('refresh_scheduler.enable_installation_monitor')">
          <el-switch
            v-model="config.refresh_scheduler_config.enable_installation_monitor"
            @change="(val: boolean) => configStore.updateConfig({ refresh_scheduler_config: { enable_installation_monitor: val } })"
          />
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('refresh_scheduler.enable_installation_monitor_tooltip')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>

        <el-form-item :label="t('refresh_scheduler.monitor_debounce_secs')">
          <el-input-number
            v-model="config.refresh_scheduler_config.monitor_debounce_secs"
            :step="1"
            :precision="0"
            :min="1"
            :max="60"
            @change="(val: number) => configStore.updateConfig({ refresh_scheduler_config: { monitor_debounce_secs: val } })"
          >
            <template #suffix>
              <span>{{ t('refresh_scheduler.seconds') }}</span>
            </template>
          </el-input-number>
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('refresh_scheduler.monitor_debounce_tooltip')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>

        <el-divider content-position="left">
          {{ t('app_config.window_interaction') }}
        </el-divider>
        <el-form-item :label="t('app_config.esc_priority_close')">
          <el-switch
            v-model="config.app_config.is_esc_hide_window_priority"
            @change="(val: boolean) => configStore.updateConfig({ app_config: { is_esc_hide_window_priority: val } })"
          />
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('app_config.esc_priority_tooltip')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>

        <el-form-item :label="t('app_config.enable_drag_window')">
          <el-switch
            v-model="config.app_config.is_enable_drag_window"
            @change="(val: boolean) => configStore.updateConfig({ app_config: { is_enable_drag_window: val } })"
          />
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('app_config.drag_window_tooltip')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>

        <el-form-item :label="t('app_config.wake_on_fullscreen')">
          <el-switch
            v-model="config.app_config.is_wake_on_fullscreen"
            @change="(val: boolean) => configStore.updateConfig({ app_config: { is_wake_on_fullscreen: val } })"
          />
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('app_config.wake_fullscreen_tooltip')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>

        <el-form-item :label="t('app_config.follow_mouse')">
          <el-switch
            v-model="config.app_config.show_pos_follow_mouse"
            @change="(val: boolean) => configStore.updateConfig({ app_config: { show_pos_follow_mouse: val } })"
          />
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('app_config.follow_mouse_tooltip')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>

        <el-form-item :label="t('app_config.space_is_enter')">
          <el-switch
            v-model="config.app_config.space_is_enter"
            @change="(val: boolean) => configStore.updateConfig({ app_config: { space_is_enter: val } })"
          />
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('app_config.space_enter_tooltip')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>

        <el-divider content-position="left">
          {{ t('app_config.advanced') }}
        </el-divider>
        <el-form-item :label="t('app_config.debug_mode')">
          <el-switch
            v-model="config.app_config.is_debug_mode"
            @change="(val: boolean) => configStore.updateConfig({ app_config: { is_debug_mode: val } })"
          />
        </el-form-item>

        <el-form-item :label="t('app_config.log_level')">
          <el-select
            v-model="config.app_config.log_level"
            style="width: 120px"
            @change="(val: 'debug' | 'info' | 'warn' | 'error') => configStore.updateConfig({ app_config: { log_level: val } })"
          >
            <el-option
              label="Debug"
              value="debug"
            />
            <el-option
              label="Info"
              value="info"
            />
            <el-option
              label="Warn"
              value="warn"
            />
            <el-option
              label="Error"
              value="error"
            />
          </el-select>
          <el-tooltip
            placement="top"
            :content="t('app_config.log_level_tooltip')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>

        <el-form-item :label="t('app_config.export_logs_button')">
          <el-button
            type="primary"
            @click="exportLogs"
          >
            {{ t('app_config.export_logs_button') }}
          </el-button>
          <el-tooltip
            placement="top"
            :content="t('app_config.export_logs_tooltip')"
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

import { QuestionFilled } from '@element-plus/icons-vue'
import { useRemoteConfigStore } from '../../../stores/remote_config'
import { storeToRefs } from 'pinia'
import { useI18n } from 'vue-i18n'
import { computed } from 'vue'
import { initializeLanguage } from '../../../i18n/index'
import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import { ElMessage } from 'element-plus'

const { t } = useI18n()
const configStore = useRemoteConfigStore()
const { config } = storeToRefs(configStore)

// 语言切换功能 - 使用computed确保响应式更新
const currentLanguage = computed({
    get: () => config.value.app_config.language,
    set: (value: string) => {
        // 使用全局语言初始化函数
        initializeLanguage(value)
        configStore.updateConfig({ app_config: { language: value } })
    },
})

const changeLanguage = (lang: string) => {
    currentLanguage.value = lang
}

// 导出日志功能
const exportLogs = async () => {
    try {
        // 生成默认文件名（带时间戳）
        const now = new Date()
        const timestamp = now.toISOString().replace(/[-:]/g, '').replace('T', '-').split('.')[0]
        const defaultFileName = `zerolaunch-logs-${timestamp}.zip`

        // 打开保存对话框
        const savePath = await save({
            defaultPath: defaultFileName,
            filters: [{
                name: 'ZIP Archive',
                extensions: ['zip'],
            }],
        })

        // 用户取消选择
        if (!savePath) {
            ElMessage.info(t('app_config.export_logs_cancelled'))
            return
        }

        // 调用后端命令导出日志
        await invoke('command_export_logs', { savePath })
        
        // 导出成功
        ElMessage.success(t('app_config.export_logs_success'))
    } catch (error) {
        // 导出失败
        const errorMessage = error instanceof Error ? error.message : String(error)
        ElMessage.error(t('app_config.export_logs_failed', { error: errorMessage }))
        console.error('导出日志失败:', error)
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

.el-question-icon {
    margin-left: 8px;
}

.el-icon {
    font-size: 18px;
    color: #606266;
}
</style>

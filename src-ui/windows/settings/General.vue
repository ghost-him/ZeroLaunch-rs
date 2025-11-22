<template>
  <div class="settings-page">
    <h2 class="page-title">
      {{ t('app_config.general_settings') }}
    </h2>
    <el-form
      label-width="auto"
      class="settings-form"
    >
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

      <el-form-item :label="t('app_config.auto_refresh_time')">
        <el-input-number
          v-model="config.app_config.auto_refresh_time"
          :step="1"
          :precision="0"
          :min="1"
          @change="(val: number) => configStore.updateConfig({ app_config: { auto_refresh_time: val } })"
        >
          <template #suffix>
            <span>{{ t('app_config.minutes') }}</span>
          </template>
        </el-input-number>
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
    </el-form>
  </div>
</template>

<script lang="ts" setup>
import { useI18n } from 'vue-i18n'
import { useRemoteConfigStore } from '../../stores/remote_config'
import { storeToRefs } from 'pinia'
import { QuestionFilled } from '@element-plus/icons-vue'
import { ref, watch } from 'vue'
import { initializeLanguage } from '../../i18n/index'

const { t } = useI18n()
const configStore = useRemoteConfigStore()
const { config } = storeToRefs(configStore)

const currentLanguage = ref(config.value.app_config.language)

watch(() => config.value.app_config.language, (newVal) => {
    currentLanguage.value = newVal
})

const changeLanguage = (val: string) => {
    configStore.updateConfig({ app_config: { language: val } })
    initializeLanguage(val)
}
</script>

<style scoped>
.settings-page {
    padding: 20px;
    height: 100%;
    overflow-y: auto;
    box-sizing: border-box;
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

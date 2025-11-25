<template>
  <div class="settings-page">
    <h2 class="page-title">
      {{ t('ui_config.window_settings') }}
    </h2>
    <div class="content-container">
      <el-form label-width="auto">
        <el-form-item :label="t('ui_config.theme_mode')">
          <el-select
            v-model="config.ui_config.theme_mode"
            @change="(val: ThemeMode) => configStore.updateConfig({ ui_config: { theme_mode: val } })"
          >
            <el-option
              :label="t('ui_config.theme_mode_system')"
              value="system"
            />
            <el-option
              :label="t('ui_config.theme_mode_light')"
              value="light"
            />
            <el-option
              :label="t('ui_config.theme_mode_dark')"
              value="dark"
            />
          </el-select>
        </el-form-item>

        <el-form-item :label="t('ui_config.vertical_position_ratio')">
          <el-input-number
            v-model="config.ui_config.vertical_position_ratio"
            placeholder="0.4"
            :min="0"
            :step="0.05"
            :max="1"
            @change="(val: number) => configStore.updateConfig({ ui_config: { vertical_position_ratio: val } })"
          />
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('ui_config.vertical_position_tooltip')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>

        <el-form-item :label="t('ui_config.search_bar_height')">
          <el-input-number
            v-model="config.ui_config.search_bar_height"
            placeholder="65"
            :min="1"
            :step="1"
            :precision="0"
            @change="(val: number) => configStore.updateConfig({ ui_config: { search_bar_height: val } })"
          >
            <template #suffix>
              <span>px</span>
            </template>
          </el-input-number>
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('ui_config.px_unit_tooltip')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>

        <el-form-item :label="t('ui_config.result_item_height')">
          <el-input-number
            v-model="config.ui_config.result_item_height"
            placeholder="62"
            :min="1"
            :step="1"
            :precision="0"
            @change="(val: number) => configStore.updateConfig({ ui_config: { result_item_height: val } })"
          >
            <template #suffix>
              <span>px</span>
            </template>
          </el-input-number>
        </el-form-item>

        <el-form-item :label="t('ui_config.footer_height')">
          <el-input-number
            v-model="config.ui_config.footer_height"
            placeholder="42"
            :min="0"
            :step="1"
            :precision="0"
            @change="(val: number) => configStore.updateConfig({ ui_config: { footer_height: val } })"
          >
            <template #suffix>
              <span>px</span>
            </template>
          </el-input-number>
        </el-form-item>

        <el-form-item :label="t('ui_config.window_width')">
          <el-input-number
            v-model="config.ui_config.window_width"
            placeholder="800"
            :min="400"
            :step="10"
            :max="2000"
            :precision="0"
            @change="(val: number) => configStore.updateConfig({ ui_config: { window_width: val } })"
          >
            <template #suffix>
              <span>px</span>
            </template>
          </el-input-number>
        </el-form-item>

        <el-form-item :label="t('ui_config.use_windows_system_radius')">
          <el-switch
            v-model="config.ui_config.use_windows_sys_control_radius"
            @change="(val: boolean) => configStore.updateConfig({ ui_config: { use_windows_sys_control_radius: val } })"
          />
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('ui_config.windows11_requirement')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>

        <el-form-item :label="t('ui_config.window_corner_radius')">
          <el-input-number
            v-model="config.ui_config.window_corner_radius"
            placeholder="10"
            :min="0"
            :step="1"
            :max="50"
            :precision="0"
            :disabled="config.ui_config.use_windows_sys_control_radius"
            @change="(val: number) => configStore.updateConfig({ ui_config: { window_corner_radius: val } })"
          >
            <template #suffix>
              <span>px</span>
            </template>
          </el-input-number>
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('ui_config.system_radius_disabled')"
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
import { useI18n } from 'vue-i18n'
import { QuestionFilled } from '@element-plus/icons-vue'
import { useRemoteConfigStore } from '../../../stores/remote_config'
import { storeToRefs } from 'pinia'
import { ThemeMode } from '../../../api/remote_config_types'

const { t } = useI18n()
const configStore = useRemoteConfigStore()
const { config } = storeToRefs(configStore)
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
    overflow-y: auto;
}

.el-question-icon {
    margin-left: 8px;
}

.el-icon {
    font-size: 18px;
    color: #606266;
}
</style>

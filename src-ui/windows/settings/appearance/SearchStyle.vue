<template>
  <div class="settings-page">
    <h2 class="page-title">
      {{ t('ui_config.search_and_result_settings') }}
    </h2>
    <div class="content-container">
      <el-form label-width="auto">
        <!-- Hints Section -->
        <el-divider content-position="left">
          {{ t('ui_config.hints') }}
        </el-divider>
        <el-form-item :label="t('ui_config.custom_search_bar_placeholder')">
          <el-input
            v-model="config.app_config.search_bar_placeholder"
            placeholder="Hello, ZeroLaunch!"
            @change="(val: string) => configStore.updateConfig({ app_config: { search_bar_placeholder: val } })"
          />
        </el-form-item>

        <el-form-item :label="t('ui_config.custom_footer_tips')">
          <el-input
            v-model="config.app_config.tips"
            placeholder="ZeroLaunch-rs v0.4.0"
            @change="(val: string) => configStore.updateConfig({ app_config: { tips: val } })"
          />
        </el-form-item>

        <!-- Color Settings Section -->
        <el-divider content-position="left">
          {{ t('ui_config.theme_color_settings') }}
        </el-divider>

        <el-form-item :label="t('ui_config.theme_mode')">
          <el-select
            v-model="config.ui_config.frontend_theme_mode"
            @change="onThemeModeChange"
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

        <el-form-item :label="t('ui_config.tray_theme_mode')">
          <el-select
            v-model="config.ui_config.tray_theme_mode"
            @change="(val: ThemeMode) => configStore.updateConfig({ ui_config: { tray_theme_mode: val } })"
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
        
        <el-tabs v-model="activeColorTab" type="card">
          <el-tab-pane :label="t('ui_config.theme_mode_light')" name="light">
            <el-form-item v-for="item in colorConfigs" :key="item.key" :label="t(item.label)">
              <el-color-picker
                v-model="config.ui_config.light_mode_colors[item.key]"
                show-alpha
                @change="(val: string) => updateColor('light_mode_colors', item.key, val)"
              />
            </el-form-item>
          </el-tab-pane>

          <el-tab-pane :label="t('ui_config.theme_mode_dark')" name="dark">
             <el-form-item v-for="item in colorConfigs" :key="item.key" :label="t(item.label)">
              <el-color-picker
                v-model="config.ui_config.dark_mode_colors[item.key]"
                show-alpha
                @change="(val: string) => updateColor('dark_mode_colors', item.key, val)"
              />
            </el-form-item>
          </el-tab-pane>
        </el-tabs>

        <!-- Search Bar Font & Layout Section -->
        <el-divider content-position="left">
          {{ t('ui_config.search_bar') }}
        </el-divider>
        <el-form-item :label="t('ui_config.search_bar_font_settings')">
          <el-select
            v-model="config.ui_config.search_bar_font_family"
            filterable
            :placeholder="t('ui_config.select_or_enter_font')"
            @change="(val: string) => configStore.updateConfig({ ui_config: { search_bar_font_family: val } })"
          >
            <el-option
              v-for="font in systemFonts"
              :key="font"
              :label="font"
              :value="font"
            >
              <span :style="{ fontFamily: font }">{{ font }}</span>
            </el-option>
          </el-select>
        </el-form-item>
        <el-form-item :label="t('ui_config.search_bar_font_size')">
          <el-input-number
            v-model="config.ui_config.search_bar_font_size"
            placeholder="50"
            :min="5"
            :step="5"
            :max="100"
            @change="(val: number) => configStore.updateConfig({ ui_config: { search_bar_font_size: val } })"
          >
            <template #suffix>
              <span>%</span>
            </template>
          </el-input-number>
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('ui_config.font_size_tooltip')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>
        <el-form-item :label="t('ui_config.search_bar_animation')">
          <el-switch
            v-model="config.ui_config.search_bar_animate"
            @change="(val: boolean) => configStore.updateConfig({ ui_config: { search_bar_animate: val } })"
          />
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('ui_config.search_bar_animation_tooltip')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>

        <!-- Result Bar Font & Layout Section -->
        <el-divider content-position="left">
          {{ t('ui_config.result_bar') }}
        </el-divider>
        <el-form-item :label="t('ui_config.result_bar_font_settings')">
          <el-select
            v-model="config.ui_config.result_item_font_family"
            filterable
            :placeholder="t('ui_config.select_or_enter_font')"
            @change="(val: string) => configStore.updateConfig({ ui_config: { result_item_font_family: val } })"
          >
            <el-option
              v-for="font in systemFonts"
              :key="font"
              :label="font"
              :value="font"
            >
              <span :style="{ fontFamily: font }">{{ font }}</span>
            </el-option>
          </el-select>
        </el-form-item>
        <el-form-item :label="t('ui_config.result_bar_font_size')">
          <el-input-number
            v-model="config.ui_config.item_font_size"
            placeholder="33"
            :min="5"
            :step="5"
            :max="100"
            @change="(val: number) => configStore.updateConfig({ ui_config: { item_font_size: val } })"
          >
            <template #suffix>
              <span>%</span>
            </template>
          </el-input-number>
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('ui_config.result_font_size_tooltip')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>
        <el-form-item :label="t('ui_config.show_launch_command')">
          <el-switch
            v-model="config.ui_config.show_launch_command"
            @change="(val: boolean) => configStore.updateConfig({ ui_config: { show_launch_command: val } })"
          />
        </el-form-item>

        <!-- Footer Bar Font & Layout Section -->
        <el-divider content-position="left">
          {{ t('ui_config.footer_bar') }}
        </el-divider>
        <el-form-item :label="t('ui_config.footer_font_settings')">
          <el-select
            v-model="config.ui_config.footer_font_family"
            filterable
            :placeholder="t('ui_config.select_or_enter_font')"
            @change="(val: string) => configStore.updateConfig({ ui_config: { footer_font_family: val } })"
          >
            <el-option
              v-for="font in systemFonts"
              :key="font"
              :label="font"
              :value="font"
            >
              <span :style="{ fontFamily: font }">{{ font }}</span>
            </el-option>
          </el-select>
        </el-form-item>
        <el-form-item :label="t('ui_config.footer_font_size')">
          <el-input-number
            v-model="config.ui_config.footer_font_size"
            placeholder="33"
            :min="5"
            :step="5"
            :max="100"
            @change="(val: number) => configStore.updateConfig({ ui_config: { footer_font_size: val } })"
          >
            <template #suffix>
              <span>%</span>
            </template>
          </el-input-number>
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('ui_config.footer_font_size_tooltip')"
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
import { rgbaToHex } from '../../../utils/color'
import { QuestionFilled } from '@element-plus/icons-vue'
import { invoke } from '@tauri-apps/api/core'
import { onMounted, ref } from 'vue'
import { useRemoteConfigStore } from '../../../stores/remote_config'
import { storeToRefs } from 'pinia'
import type { UiThemeColorPalette, ThemeMode } from '../../../api/remote_config_types'

const { t } = useI18n()

const configStore = useRemoteConfigStore()
const { config } = storeToRefs(configStore)

const activeColorTab = ref('light')

const onThemeModeChange = (val: ThemeMode) => {
    configStore.updateConfig({ ui_config: { frontend_theme_mode: val } })
    if (val === 'dark') {
        activeColorTab.value = 'dark'
    } else if (val === 'light') {
        activeColorTab.value = 'light'
    }
}

type ColorKey = keyof UiThemeColorPalette

const colorConfigs: { key: ColorKey; label: string }[] = [
    { key: 'program_background_color', label: 'ui_config.overall_background_color' },
    { key: 'search_bar_background_color', label: 'ui_config.search_bar_status_bar_background' },
    { key: 'selected_item_color', label: 'ui_config.result_bar_highlight_color' },
    { key: 'search_bar_font_color', label: 'ui_config.search_bar_font_color' },
    { key: 'search_bar_placeholder_font_color', label: 'ui_config.search_bar_placeholder_font_color' },
    { key: 'item_font_color', label: 'ui_config.result_bar_font_color' },
    { key: 'footer_font_color', label: 'ui_config.footer_font_color' },
]

const updateColor = (mode: 'light_mode_colors' | 'dark_mode_colors', key: ColorKey, val: string | null) => {
    if (!val) return
    configStore.updateConfig({
        ui_config: {
            [mode]: {
                [key]: rgbaToHex(val)
            }
        }
    })
}

const systemFonts = ref<string[]>([
    'Default', 'Arial', 'Helvetica', 'Times New Roman', 'Courier New', 'SimSun', 'Microsoft YaHei',
])

// 加载系统字体的方法
const loadSystemFonts = async () => {
    try {
        const fonts = await invoke('command_get_system_fonts')
        if (Array.isArray(fonts) && fonts.length > 0) {
            systemFonts.value = fonts
        }
    } catch (error) {
        console.error(t('ui_config.get_system_fonts_failed'), error)
    }
}

onMounted(async () => {
    await loadSystemFonts()
})
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

<template>
  <div class="settings-page">
    <div class="shortcut-header">
      <h2 class="page-title">
        {{ t('shortcut.title') }}
      </h2>
      <div class="shortcut-actions">
        <el-button
          type="primary"
          :disabled="is_saving"
          @click="edit_shortcut_config"
        >
          <el-icon>
            <Edit v-if="!is_editing" />
            <Close v-else />
          </el-icon>
          {{ is_editing ? t('shortcut.cancel_edit') : t('shortcut.start_setting') }}
        </el-button>
        <el-button
          type="success"
          :loading="is_saving"
          :disabled="!is_editing"
          @click="save_shortcut_config"
        >
          <el-icon>
            <Check />
          </el-icon>
          {{ t('shortcut.save_settings') }}
        </el-button>
        <el-button
          type="warning"
          :disabled="!is_editing"
          @click="resetAllShortcuts"
        >
          <el-icon>
            <RefreshLeft />
          </el-icon>
          {{ t('shortcut.reset_all') }}
        </el-button>
      </div>
    </div>

    <div class="content-container">
      <el-divider content-position="left">
        {{ t('shortcut.system_shortcuts') }}
      </el-divider>
      <div class="shortcut-settings-container">
        <el-form
          label-position="top"
          class="shortcut-form"
        >
          <el-form-item
            v-for="item in shortcutItems"
            :key="item.key"
            class="shortcut-form-item"
          >
            <div class="shortcut-item-header">
              <div class="shortcut-label">
                <el-icon>
                  <component :is="item.icon" />
                </el-icon>
                <span>{{ item.label }}</span>
              </div>
              <div class="header-actions">
                <el-tooltip
                  :content="t('shortcut.reset')"
                  placement="top"
                  effect="light"
                >
                  <el-button
                    link
                    class="reset-icon-btn"
                    :disabled="!is_editing"
                    @click="resetShortcut(item.key)"
                  >
                    <el-icon>
                      <RefreshRight />
                    </el-icon>
                  </el-button>
                </el-tooltip>
                <el-tooltip
                  :content="item.tooltip"
                  placement="top"
                  effect="light"
                >
                  <el-icon class="info-icon">
                    <InfoFilled />
                  </el-icon>
                </el-tooltip>
              </div>
            </div>

            <div class="shortcut-item-content">
              <ShortcutInput
                v-model="dirty_shortcut_config[item.key]"
                :disabled="!is_editing"
                :placeholder="item.placeholder"
              />
            </div>
          </el-form-item>
        </el-form>
      </div>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { default_shortcut_config, ShortcutConfig } from '../../api/remote_config_types'
import ShortcutInput from './components/ShortcutInput.vue'
import { onUnmounted, ref, markRaw } from 'vue'
import { onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useRemoteConfigStore } from '../../stores/remote_config'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useI18n } from 'vue-i18n'
import {
    Search, ArrowLeft, ArrowRight, ArrowUp, ArrowDown,
    InfoFilled, Edit, Close, Check, RefreshLeft,
    RefreshRight, Files,
} from '@element-plus/icons-vue'
import type { Component } from 'vue'
const { t } = useI18n()
const configStore = useRemoteConfigStore()
const is_editing = ref<boolean>(false)
const is_saving = ref<boolean>(false)
const d_shortcut_config: ShortcutConfig = default_shortcut_config()
const dirty_shortcut_config = ref<ShortcutConfig>({ ...configStore.config.shortcut_config })

// 定义快捷键标签和描述
type ShortcutKey = keyof ShortcutConfig;
const shortcutItems = ref<Array<{
    key: keyof ShortcutConfig;
    icon: Component;
    label: string;
    tooltip: string;
    placeholder: string;
}>>([
    {
        key: 'open_search_bar',
        icon: markRaw(Search),
        label: t('shortcut.open_search_bar'),
        tooltip: t('shortcut.open_search_bar_tooltip'),
        placeholder: t('shortcut.open_search_bar_placeholder'),
    },
    {
        key: 'switch_to_everything',
        icon: markRaw(Files),
        label: t('shortcut.switch_to_everything'),
        tooltip: t('shortcut.switch_to_everything_tooltip'),
        placeholder: t('shortcut.switch_to_everything_placeholder'),
    },
    {
        key: 'arrow_left',
        icon: markRaw(ArrowLeft),
        label: t('shortcut.arrow_left'),
        tooltip: t('shortcut.arrow_left_tooltip'),
        placeholder: t('shortcut.arrow_left_placeholder'),
    },
    {
        key: 'arrow_right',
        icon: markRaw(ArrowRight),
        label: t('shortcut.arrow_right'),
        tooltip: t('shortcut.arrow_right_tooltip'),
        placeholder: t('shortcut.arrow_right_placeholder'),
    },
    {
        key: 'arrow_up',
        icon: markRaw(ArrowUp),
        label: t('shortcut.arrow_up'),
        tooltip: t('shortcut.arrow_up_tooltip'),
        placeholder: t('shortcut.arrow_up_placeholder'),
    },
    {
        key: 'arrow_down',
        icon: markRaw(ArrowDown),
        label: t('shortcut.arrow_down'),
        tooltip: t('shortcut.arrow_down_tooltip'),
        placeholder: t('shortcut.arrow_down_placeholder'),
    },
])

const edit_shortcut_config = async () => {
    if (is_editing.value) {
        // 取消编辑，恢复原始配置
        dirty_shortcut_config.value = { ...configStore.config.shortcut_config }
        is_editing.value = false
        try {
            await invoke('command_register_all_shortcut')
            ElMessage.info(t('shortcut.edit_cancelled'))
        } catch (error) {
            handleError(t('shortcut.restore_failed') + error)
        }
    } else {
        try {
            await invoke('command_unregister_all_shortcut')
            is_editing.value = true
            ElMessage.success({
                message: t('shortcut.edit_mode_entered'),
                duration: 2000,
            })
        } catch (error) {
            handleError(t('shortcut.unbind_failed') + error)
        }
    }
}

const save_shortcut_config = async () => {
    is_saving.value = true
    try {
        await configStore.updateConfig({ shortcut_config: dirty_shortcut_config.value })
        await configStore.syncConfig()
        is_editing.value = false
        ElMessage.success({
            message: t('shortcut.settings_saved'),
            type: 'success',
            duration: 2000,
        })
    } catch (error) {
        handleError(t('shortcut.save_failed') + error)
        try {
            await invoke('command_register_all_shortcut')
        } catch (e) {
            handleError(t('shortcut.restore_default_failed') + e)
        }
    } finally {
        is_saving.value = false
    }
}

const resetShortcut = (key: ShortcutKey) => {
    dirty_shortcut_config.value[key] = d_shortcut_config[key]
    ElMessage.info(t('shortcut.reset_to_default'))
}

const resetAllShortcuts = async () => {
    try {
        await ElMessageBox.confirm(
            t('shortcut.reset_all_confirm'),
            t('shortcut.reset_confirm_title'),
            {
                confirmButtonText: t('shortcut.confirm_reset'),
                cancelButtonText: t('shortcut.cancel'),
                type: 'warning',
            },
        )
        dirty_shortcut_config.value = { ...d_shortcut_config }
        ElMessage.success(t('shortcut.all_reset_success'))
    } catch {
        // 用户取消操作
    }
}

const handleError = (error: string) => {
    ElMessage({
        showClose: true,
        message: error,
        type: 'error',
        duration: 5000,
    })
}

onMounted(async () => {
    // 初始化逻辑
})

onUnmounted(async () => {
    if (is_editing.value) {
        await invoke('command_register_all_shortcut')
        is_editing.value = false
    }
})
</script>

<style scoped>
.settings-page {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 20px;
    box-sizing: border-box;
}

.page-title {
    margin: 0;
    font-size: 24px;
    font-weight: 500;
    color: var(--el-text-color-primary);
}

.content-container {
    flex: 1;
    overflow-y: auto;
    padding-right: 10px;
}

.shortcut-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
    flex-shrink: 0;
}

.shortcut-actions {
    display: flex;
    gap: 12px;
}

.shortcut-settings-container {
    margin-top: 20px;
}

.shortcut-form {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
    gap: 20px;
}

.shortcut-form-item {
    background-color: var(--el-bg-color);
    padding: 16px;
    border-radius: 8px;
    box-shadow: 0 2px 12px 0 rgba(0, 0, 0, 0.05);
    transition: all 0.3s;
    margin: 0;
}

.shortcut-form-item:hover {
    box-shadow: 0 4px 16px 0 rgba(0, 0, 0, 0.1);
}

.shortcut-item-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;
}

.shortcut-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 500;
    color: var(--el-text-color-primary);
}

.header-actions {
    display: flex;
    align-items: center;
    gap: 12px;
}

.info-icon {
    color: var(--el-color-info);
    cursor: pointer;
    font-size: 16px;
}

.reset-icon-btn {
    padding: 0;
    height: auto;
    font-size: 16px;
    color: var(--el-text-color-secondary);
    transition: color 0.2s;
}

.reset-icon-btn:hover:not(:disabled) {
    color: var(--el-color-primary);
}

.reset-icon-btn:disabled {
    color: var(--el-text-color-disabled);
}

.shortcut-item-content {
    display: flex;
    align-items: center;
}

:deep(.shortcut-input) {
    margin-bottom: 0;
    width: 100%;
}

:deep(.key-display) {
    flex: 1;
    width: auto;
    min-width: 0;
}

:deep(.el-form-item__content) {
    display: flex;
    flex-direction: column;
    align-items: stretch;
}

:deep(.el-divider__text) {
    font-size: 16px;
    font-weight: 500;
    color: var(--el-text-color-secondary);
}
</style>

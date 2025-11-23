<template>
  <div class="settings-layout">
    <div class="sidebar">
      <div class="header">
        <img
          src="../assets/icon.svg"
          alt="Logo"
          class="logo"
        >
        <span class="title">{{ t('settings.title') }}</span>
      </div>

      <el-scrollbar>
        <el-menu
          :default-active="activeMenu"
          class="settings-menu"
          :collapse="false"
          @select="handleMenuSelect"
        >
          <el-menu-item index="/setting_window/general">
            <el-icon>
              <Setting />
            </el-icon>
            <span>{{ t('settings.menu.general') }}</span>
          </el-menu-item>

          <el-sub-menu index="/setting_window/appearance">
            <template #title>
              <el-icon>
                <Brush />
              </el-icon>
              <span>{{ t('settings.menu.appearance') }}</span>
            </template>
            <el-menu-item index="/setting_window/appearance/search">
              {{
                t('ui_config.search_and_result_settings') }}
            </el-menu-item>
            <el-menu-item index="/setting_window/appearance/background">
              {{
                t('ui_config.background_image_settings')
              }}
            </el-menu-item>
            <el-menu-item index="/setting_window/appearance/window">
              {{ t('ui_config.window_settings')
              }}
            </el-menu-item>
          </el-sub-menu>

          <el-sub-menu index="/setting_window/programs">
            <template #title>
              <el-icon>
                <Search />
              </el-icon>
              <span>{{ t('settings.menu.program_search') }}</span>
            </template>
            <el-menu-item index="/setting_window/programs/paths">
              {{ t('program_index.set_search_path')
              }}
            </el-menu-item>
            <el-menu-item index="/setting_window/programs/blocklist">
              {{ t('program_index.set_blocked_paths')
              }}
            </el-menu-item>
            <el-menu-item index="/setting_window/programs/keywords">
              {{ t('program_index.set_fixed_offset')
              }}
            </el-menu-item>
            <el-menu-item index="/setting_window/programs/aliases">
              {{ t('program_index.setting_alias')
              }}
            </el-menu-item>
            <el-menu-item index="/setting_window/programs/advanced">
              {{ t('program_index.extra_settings')
              }}
            </el-menu-item>
          </el-sub-menu>

          <el-menu-item index="/setting_window/icons">
            <el-icon>
              <Picture />
            </el-icon>
            <span>{{ t('settings.icon_management') }}</span>
          </el-menu-item>

          <el-sub-menu index="/setting_window/search">
            <template #title>
              <el-icon>
                <List />
              </el-icon>
              <span>{{ t('settings.menu.other_search') }}</span>
            </template>
            <el-menu-item index="/setting_window/search/web">
              {{ t('settings.custom_web_search')
              }}
            </el-menu-item>
            <el-menu-item index="/setting_window/search/custom">
              {{ t('settings.custom_command_search')
              }}
            </el-menu-item>
            <el-menu-item index="/setting_window/search/builtin">
              {{ t('settings.builtin_command_settings')
              }}
            </el-menu-item>
          </el-sub-menu>

          <el-menu-item index="/setting_window/config">
            <el-icon>
              <Connection />
            </el-icon>
            <span>{{ t('settings.menu.remote_management') }}</span>
          </el-menu-item>

          <el-menu-item index="/setting_window/shortcuts">
            <el-icon>
              <Key />
            </el-icon>
            <span>{{ t('settings.menu.shortcuts') }}</span>
          </el-menu-item>

          <el-menu-item index="/setting_window/about">
            <el-icon>
              <InfoFilled />
            </el-icon>
            <span>{{ t('settings.menu.about') }}</span>
          </el-menu-item>

          <el-menu-item
            v-if="isDebugMode"
            index="/setting_window/debug"
          >
            <el-icon>
              <Monitor />
            </el-icon>
            <span>{{ t('settings.debug_mode') }}</span>
          </el-menu-item>
        </el-menu>
      </el-scrollbar>

      <div class="footer-actions">
        <el-button
          type="primary"
          :loading="isSaving"
          :disabled="isSaveDisabled"
          class="save-btn"
          @click="saveConfig"
        >
          {{ t('settings.save_config') }}
        </el-button>
      </div>
    </div>

    <div class="content">
      <router-view v-slot="{ Component }">
        <transition
          name="fade"
          mode="out-in"
        >
          <component :is="Component" />
        </transition>
      </router-view>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { computed, ref, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useRemoteConfigStore } from '../stores/remote_config'
import { storeToRefs } from 'pinia'
import { listen, UnlistenFn } from '@tauri-apps/api/event'
import { ElMessage } from 'element-plus'
import { initializeLanguage } from '../i18n/index'
import {
    Setting,
    Brush,
    Search,
    List,
    Connection,
    Key,
    InfoFilled,
    Monitor,
    Picture,
} from '@element-plus/icons-vue'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const configStore = useRemoteConfigStore()
const { config } = storeToRefs(configStore)

const activeMenu = computed(() => route.path)
const isDebugMode = computed(() => config.value.app_config.is_debug_mode)
const isSaving = ref(false)

const isDirty = computed(() => Object.keys(configStore.dirtyConfig).length > 0)
const restrictedPaths = [
    '/setting_window/config',
    '/setting_window/shortcuts',
    '/setting_window/about',
    '/setting_window/debug',
]
const isRestrictedPage = computed(() => restrictedPaths.some(path => route.path.startsWith(path)))

const isSaveDisabled = computed(() => {
    return !isDirty.value || isRestrictedPage.value
})

const handleMenuSelect = (index: string) => {
    if (restrictedPaths.some(path => index.startsWith(path))) {
        if (isDirty.value) {
            ElMessage.warning(t('settings.please_save_first'))
            return
        }
    }
    router.push(index)
}

const saveConfig = async () => {
    isSaving.value = true
    try {
        await configStore.syncConfig()
        ElMessage.success(t('settings.config_saved'))
    } catch (error) {
        ElMessage.error(t('settings.save_failed'))
    } finally {
        isSaving.value = false
    }
}

let unlisten: Array<UnlistenFn | null> = []

onMounted(async () => {
    await configStore.loadConfig()
    initializeLanguage(config.value.app_config.language)

    unlisten.push(await listen('emit_update_setting_window_config', async () => {
        await configStore.loadConfig()
        initializeLanguage(config.value.app_config.language)
    }))
})

onUnmounted(() => {
    unlisten.forEach(fn => fn && fn())
    unlisten = []
})
</script>

<style scoped>
.settings-layout {
    display: flex;
    width: 100%;
    height: 100vh;
    background-color: #fff;
    overflow: hidden;
}

.sidebar {
    width: 240px;
    background-color: #f5f7fa;
    display: flex;
    flex-direction: column;
    border-right: 1px solid #e6e6e6;
    flex-shrink: 0;
}

.header {
    padding: 20px;
    display: flex;
    align-items: center;
    border-bottom: 1px solid #e6e6e6;
}

.logo {
    width: 28px;
    height: 28px;
    margin-right: 10px;
}

.title {
    font-size: 18px;
    font-weight: 600;
    color: #303133;
}

.settings-menu {
    border-right: none;
    background-color: transparent;
}

.footer-actions {
    padding: 16px;
    border-top: 1px solid #e6e6e6;
    display: flex;
    justify-content: center;
    background-color: #f5f7fa;
}

.save-btn {
    width: 100%;
}

.content {
    flex: 1;
    padding: 0;
    overflow: hidden;
    background-color: #fff;
    position: relative;
}

/* Transition */
.fade-enter-active,
.fade-leave-active {
    transition: opacity 0.025s ease;
}

.fade-enter-from,
.fade-leave-to {
    opacity: 0;
}
</style>

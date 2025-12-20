<template>
  <el-dialog
    v-model="visible"
    :title="t('settings.import_bookmarks')"
    width="500px"
    destroy-on-close
    @open="detectBrowsers"
  >
    <div v-loading="loading" class="browser-list">
      <div v-if="browsers.length === 0 && !loading" class="no-browser">
        {{ t('settings.no_browser_detected') }}
      </div>
      
      <div 
        v-for="browser in browsers" 
        :key="browser.name" 
        class="browser-item"
        @click="selectBrowser(browser)"
      >
        <div class="browser-icon">
          <el-icon><Monitor /></el-icon>
        </div>
        <div class="browser-info">
          <div class="browser-name">{{ browser.name }}</div>
          <div class="browser-path">{{ browser.bookmarks_path }}</div>
        </div>
        <el-icon><ArrowRight /></el-icon>
      </div>
    </div>
  </el-dialog>
</template>

<script lang="ts" setup>
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { Monitor, ArrowRight } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'

const props = defineProps<{
  modelValue: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
  (e: 'import', bookmarks: Array<{ title: string, url: string }>): void
}>()

const { t } = useI18n()
const loading = ref(false)
const browsers = ref<Array<{ name: string, bookmarks_path: string }>>([])

const visible = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val)
})

const detectBrowsers = async () => {
  loading.value = true
  try {
    browsers.value = await invoke('detect_installed_browsers')
  } catch (e) {
    console.error('Failed to detect browsers:', e)
    ElMessage.error(t('settings.detect_browser_failed'))
  } finally {
    loading.value = false
  }
}

const selectBrowser = async (browser: { name: string, bookmarks_path: string }) => {
  loading.value = true
  try {
    const bookmarks = await invoke<Array<{ title: string, url: string }>>('read_browser_bookmarks', {
      bookmarksPath: browser.bookmarks_path
    })
    
    if (bookmarks.length === 0) {
      ElMessage.warning(t('settings.no_bookmarks_found'))
      return
    }

    emit('import', bookmarks)
    visible.value = false
    ElMessage.success(t('settings.import_success', { count: bookmarks.length }))
  } catch (e) {
    console.error('Failed to read bookmarks:', e)
    ElMessage.error(t('settings.read_bookmarks_failed'))
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
.browser-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-height: 100px;
}

.browser-item {
  display: flex;
  align-items: center;
  padding: 12px;
  border: 1px solid var(--el-border-color);
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
}

.browser-item:hover {
  background-color: var(--el-fill-color-light);
  border-color: var(--el-color-primary);
}

.browser-icon {
  font-size: 24px;
  margin-right: 12px;
  display: flex;
  align-items: center;
  color: var(--el-text-color-secondary);
}

.browser-info {
  flex: 1;
  overflow: hidden;
}

.browser-name {
  font-weight: bold;
  font-size: 16px;
  color: var(--el-text-color-primary);
}

.browser-path {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.no-browser {
  text-align: center;
  color: var(--el-text-color-secondary);
  padding: 20px;
}
</style>

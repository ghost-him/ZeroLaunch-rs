<template>
  <div class="about-view">
    <h3 class="about-name">ZeroLaunch-rs</h3>
    <n-text depth="3" class="about-version">v{{ version }}</n-text>

    <div class="about-update">
      <n-text depth="3" class="about-update-hint">{{ updateHint }}</n-text>
      <n-button size="small" :loading="checking" @click="runUpdateCheck">
        {{ t('settings.aboutPage.checkUpdate') }}
      </n-button>
      <n-button
        v-if="updateStatus?.hasUpdate && updateStatus.releaseUrl"
        size="small"
        secondary
        @click="openReleasePage"
      >
        {{ t('settings.aboutPage.viewRelease') }}
      </n-button>
    </div>

    <p class="about-tagline">{{ t('settings.aboutPage.tagline') }}</p>

    <div class="about-info">
      <div v-for="item in infoRows" :key="item.label" class="info-row">
        <span class="info-label">{{ item.label }}</span>
        <span class="info-value">{{ item.value }}</span>
      </div>
    </div>

    <div class="about-links">
      <a
        v-for="link in links"
        :key="link.label"
        class="about-link"
        href="#"
        @click.prevent="openLink(link.url)"
      >{{ link.label }}</a>
    </div>

    <div class="about-export">
      <n-text depth="3" class="about-export-hint">{{ t('settings.aboutPage.exportLogsHint') }}</n-text>
      <n-button size="small" :loading="exporting" @click="exportLogs">
        {{ t('settings.aboutPage.exportLogs') }}
      </n-button>
    </div>

    <n-text depth="3" class="about-copyright">{{ t('settings.aboutPage.copyright') }}</n-text>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { NButton, NText, useMessage } from 'naive-ui'
import {
  exportLogs as invokeExportLogs,
  getUpdateStatus,
  checkUpdate as invokeCheckUpdate,
  openExternal,
  pickLogArchivePath,
  type UpdateStatus,
} from '@/bridge/commands'
import { formatErrorMessage } from '@/utils/errors'

defineProps<{ version: string }>()

const { t } = useI18n()
const message = useMessage()

/** 导出中状态：按钮 loading，防止重复触发。 */
const exporting = ref(false)

/** 关于页信息行：技术栈 / 开源协议 / 作者。 */
const infoRows = computed(() => [
  { label: t('settings.aboutPage.stackLabel'), value: t('settings.aboutPage.stackValue') },
  { label: t('settings.aboutPage.licenseLabel'), value: t('settings.aboutPage.licenseValue') },
  { label: t('settings.aboutPage.authorLabel'), value: t('settings.aboutPage.authorValue') },
])

/** 关于页资源链接：点击后用系统浏览器打开。 */
const links = computed(() => [
  { label: t('settings.aboutPage.website'), url: 'https://zerolaunch.ghost-him.com' },
  { label: t('settings.aboutPage.github'), url: 'https://github.com/ghost-him/ZeroLaunch-rs' },
  { label: t('settings.aboutPage.gitee'), url: 'https://gitee.com/ghost-him/ZeroLaunch-rs' },
  { label: t('settings.aboutPage.wiki'), url: 'https://github.com/ghost-him/ZeroLaunch-rs/wiki' },
  { label: t('settings.aboutPage.issues'), url: 'https://github.com/ghost-him/ZeroLaunch-rs/issues' },
])

/** 用系统浏览器打开外链，失败时提示。 */
function openLink(url: string) {
  void openExternal(url).catch(() => {
    message.error(t('settings.aboutPage.openFailed'))
  })
}

/**
 * 选择 zip 落点并导出当前日志。
 * 用户取消保存对话框时不发起导出；成功提示目标路径，失败提示后端错误信息。
 */
async function exportLogs() {
  exporting.value = true
  try {
    const destPath = await pickLogArchivePath(t('settings.aboutPage.exportLogsFilter'))
    if (!destPath) return
    await invokeExportLogs(destPath)
    message.success(t('settings.aboutPage.exportLogsSuccess', { path: destPath }))
  } catch (e) {
    message.error(t('settings.aboutPage.exportLogsFailed', { message: formatErrorMessage(e) }))
  } finally {
    exporting.value = false
  }
}

/** 最近一次更新检查结果；null 表示本次会话尚未取到（挂载时读共享状态）。 */
const updateStatus = ref<UpdateStatus | null>(null)
/** 检查中状态：按钮 loading，防止重复触发。 */
const checking = ref(false)

/** 版本检查状态文案（检查中 / 可更新 / 已是最新 / 失败）。 */
const updateHint = computed(() => {
  if (checking.value) return t('settings.aboutPage.checking')
  const status = updateStatus.value
  if (!status) return ''
  if (status.hasUpdate && status.latestVersion) {
    return t('settings.aboutPage.updateAvailable', { version: status.latestVersion })
  }
  if (status.error) return t('settings.aboutPage.checkFailed')
  return t('settings.aboutPage.upToDate', { version: status.currentVersion })
})

/** 主动检查新版本；结果只在本页展示，不自动打开浏览器。 */
async function runUpdateCheck() {
  checking.value = true
  try {
    updateStatus.value = await invokeCheckUpdate()
  } catch (e) {
    message.error(formatErrorMessage(e))
  } finally {
    checking.value = false
  }
}

/** 在新窗口打开最新版本的发布页。 */
function openReleasePage() {
  const url = updateStatus.value?.releaseUrl
  if (url) openLink(url)
}

/** 挂载时读取托盘自动检查共享的状态，避免重复请求 GitHub。 */
onMounted(async () => {
  try {
    updateStatus.value = await getUpdateStatus()
  } catch {
    updateStatus.value = null
  }
})
</script>

<style scoped>
/* 弹性填充链：settings-content(flex column) → about-view(flex:1 + 内层滚动) */
.about-view {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 32px;
  overflow-y: auto;
}

.about-name {
  margin: 0;
  font-size: var(--font-size-xl);
  font-weight: 600;
  color: var(--text-color);
  flex-shrink: 0;
}

.about-version {
  font-size: var(--font-size-sm);
  flex-shrink: 0;
}

/* 更新检查行：状态文案占满剩余宽度，按钮右对齐 */
.about-update {
  display: flex;
  align-items: center;
  gap: 12px;
  box-sizing: border-box;
  width: 100%;
  max-width: 420px;
  padding: 8px 14px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  background-color: var(--bg-color-secondary);
  flex-shrink: 0;
}

.about-update-hint {
  flex: 1;
  min-width: 0;
  font-size: var(--font-size-sm);
  line-height: 1.4;
}

.about-tagline {
  margin: 0;
  color: var(--text-color-secondary);
  font-size: var(--font-size-md);
  text-align: center;
  flex-shrink: 0;
}

.about-info {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 8px;
  flex-shrink: 0;
}

.info-row {
  display: flex;
  gap: 12px;
}

.info-label {
  width: 72px;
  text-align: right;
  color: var(--text-color-secondary);
  font-size: var(--font-size-md);
}

.info-value {
  color: var(--text-color);
  font-size: var(--font-size-md);
}

.about-links {
  display: flex;
  gap: 16px;
  margin-top: 12px;
  flex-shrink: 0;
}

.about-link {
  color: var(--primary-color);
  font-size: var(--font-size-md);
  text-decoration: none;
  cursor: pointer;
}

.about-link:hover {
  text-decoration: underline;
}

.about-copyright {
  margin-top: 12px;
  font-size: var(--font-size-sm);
  flex-shrink: 0;
}

.about-export {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  box-sizing: border-box;
  width: 100%;
  max-width: 420px;
  margin-top: 20px;
  padding: 10px 14px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  background-color: var(--bg-color-secondary);
  flex-shrink: 0;
}

.about-export-hint {
  font-size: var(--font-size-sm);
  line-height: 1.4;
}
</style>

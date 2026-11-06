<template>
  <div class="about-view">
    <div class="about-shell">
      <!-- 头部：应用图标 + 名称 + 版本徽标 + 标语 -->
      <header class="about-hero">
        <img class="about-logo" :src="appIcon" alt="" />
        <div class="about-hero-text">
          <div class="about-title-row">
            <h3 class="about-name">ZeroLaunch-rs</h3>
            <n-tag size="small" round :bordered="false" type="primary">v{{ version }}</n-tag>
          </div>
          <p class="about-tagline">{{ t('settings.aboutPage.tagline') }}</p>
        </div>
      </header>

      <!-- 版本更新 -->
      <section class="about-card">
        <div class="card-text">
          <span class="card-title">{{ t('settings.aboutPage.updateTitle') }}</span>
          <span class="card-hint">{{ updateHint }}</span>
        </div>
        <div class="card-actions">
          <n-button size="small" :loading="checking" @click="runUpdateCheck">
            {{ t('settings.aboutPage.checkUpdate') }}
          </n-button>
          <n-button
            v-if="updateStatus?.hasUpdate && updateStatus.releaseUrl"
            size="small"
            type="primary"
            secondary
            @click="openReleasePage"
          >
            {{ t('settings.aboutPage.viewRelease') }}
          </n-button>
        </div>
      </section>

      <!-- 项目信息：三列元数据 + 资源链接 -->
      <section class="about-card about-card-column">
        <div class="about-meta">
          <div v-for="item in infoRows" :key="item.label" class="meta-item">
            <span class="meta-label">{{ item.label }}</span>
            <span class="meta-value">{{ item.value }}</span>
          </div>
        </div>
        <div class="about-links">
          <n-button
            v-for="link in links"
            :key="link.label"
            size="small"
            type="primary"
            text
            @click="openLink(link.url)"
          >
            {{ link.label }}
          </n-button>
        </div>
      </section>

      <!-- 日志导出 -->
      <section class="about-card">
        <div class="card-text">
          <span class="card-title">{{ t('settings.aboutPage.logsTitle') }}</span>
          <span class="card-hint">{{ t('settings.aboutPage.exportLogsHint') }}</span>
        </div>
        <div class="card-actions">
          <n-button size="small" :loading="exporting" @click="exportLogs">
            {{ t('settings.aboutPage.exportLogs') }}
          </n-button>
        </div>
      </section>

      <p class="about-copyright">{{ t('settings.aboutPage.copyright') }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { NButton, NTag, useMessage } from 'naive-ui'
import appIcon from '../../utils/icon.svg'
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
/* 顶部对齐的滚动容器：与设置页其它页面一致，内容限宽居中 */
.about-view {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 20px 24px 28px;
}

.about-shell {
  display: flex;
  flex-direction: column;
  gap: 12px;
  width: 100%;
  max-width: 720px;
  margin: 0 auto;
}

.about-hero {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 2px 2px 4px;
}

.about-logo {
  width: 44px;
  height: 44px;
  flex-shrink: 0;
}

.about-hero-text {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.about-title-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.about-name {
  margin: 0;
  font-size: var(--font-size-xl);
  font-weight: 600;
  line-height: 1.2;
  color: var(--text-color);
}

.about-tagline {
  margin: 0;
  font-size: var(--font-size-md);
  color: var(--text-color-secondary);
}

/* 卡片：状态行（文案在左、操作在右）与信息卡（纵向排布）共用描边与圆角 */
.about-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 14px 16px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
}

.about-card-column {
  flex-direction: column;
  align-items: stretch;
  gap: 12px;
}

.card-text {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.card-title {
  font-size: var(--font-size-base);
  font-weight: 600;
  color: var(--text-color);
}

.card-hint {
  font-size: var(--font-size-md);
  line-height: 1.5;
  color: var(--text-color-secondary);
}

/* 状态文案为空时（会话内尚未取到检查结果）不占位，保持行高一致由 title 承担 */
.card-hint:empty {
  display: none;
}

.card-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

/* 项目信息：标签在上、值在下的三列栅格 */
.about-meta {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 12px;
}

.meta-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.meta-label {
  font-size: var(--font-size-sm);
  color: var(--text-color-secondary);
}

.meta-value {
  font-size: var(--font-size-base);
  color: var(--text-color);
  word-break: break-word;
}

/* 资源链接：与元数据同一张卡片，用分隔线区分层次 */
.about-links {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 2px 14px;
  padding-top: 10px;
  border-top: 1px solid var(--border-color);
}

.about-copyright {
  margin: 4px 0 0;
  text-align: center;
  font-size: var(--font-size-sm);
  color: var(--text-color-secondary);
}
</style>

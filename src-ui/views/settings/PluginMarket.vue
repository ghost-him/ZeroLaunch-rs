<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  NAlert, NButton, NCard, NEmpty, NModal, NSpace, NSpin, NText,
  useDialog, useMessage,
} from 'naive-ui'
import { open } from '@tauri-apps/plugin-shell'
import { marketList, marketInstall } from '@/bridge/commands'
import type { MarketRepo } from '@/bridge/commands'
import type { BridgeError } from '@/bridge/commands'

const { t } = useI18n()
const message = useMessage()
const dialog = useDialog()

const loading = ref(false)
const repos = ref<MarketRepo[]>([])
const loadError = ref<string | null>(null)
/** 正在安装的仓库（按 fullName 记录，按钮 loading + 防重入）。 */
const installingRepo = ref<string | null>(null)
/** 待确认安装的仓库。 */
const pendingInstall = ref<MarketRepo | null>(null)

/** 错误信息提取（BridgeError message + traceId）。 */
function errorText(e: unknown): string {
  const err = e as BridgeError
  return err.message + (err.traceId ? ` (trace: ${err.traceId})` : '')
}

/** 安装确认弹窗显隐：由 pendingInstall 派生（v-model 需可写引用，不能绑表达式）。 */
const showInstallConfirm = computed({
  get: () => pendingInstall.value !== null,
  set: (v: boolean) => {
    if (!v) pendingInstall.value = null
  },
})

/** 拉取市场仓库列表。 */
async function loadMarket() {
  loading.value = true
  loadError.value = null
  try {
    repos.value = await marketList()
  } catch (e) {
    loadError.value = errorText(e)
  } finally {
    loading.value = false
  }
}

/** 打开仓库主页（webview 内须走 shell 插件；仅放行 http/https）。 */
function openRepo(url: string) {
  let parsed: URL
  try {
    parsed = new URL(url)
  } catch {
    return
  }
  if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') return
  void open(parsed.toString()).catch(() => {
    message.error(t('settings.pluginMarket.installFailed'))
  })
}

/** 点击「安装」：先弹确认（下载并安装仓库最新发布）。 */
function requestInstall(repo: MarketRepo) {
  pendingInstall.value = repo
}

/** 执行市场安装；已安装时询问覆盖。 */
async function runInstall(repo: MarketRepo, overwrite: boolean) {
  installingRepo.value = repo.fullName
  try {
    await marketInstall(repo.fullName, overwrite)
    message.success(t('settings.pluginMarket.installSuccess'))
    pendingInstall.value = null
  } catch (e) {
    const err = e as BridgeError
    if (err.code === 'ALREADY_INSTALLED') {
      pendingInstall.value = null
      dialog.warning({
        title: t('settings.thirdPartyPlugins.overwriteDialogTitle'),
        content: t('settings.pluginMarket.overwriteConfirmContent', { name: repo.fullName }),
        positiveText: t('settings.pluginMarket.overwriteConfirmPositive'),
        negativeText: t('settings.thirdPartyPlugins.installConfirmNegative'),
        onPositiveClick: async () => {
          await runInstall(repo, true)
        },
      })
    } else {
      message.error(t('settings.pluginMarket.installFailed') + ': ' + errorText(e))
    }
  } finally {
    installingRepo.value = null
  }
}

/** 确认弹窗里的安装：防重入后再执行。 */
function confirmInstall(repo: MarketRepo) {
  if (installingRepo.value) return
  void runInstall(repo, false)
}

/** 安装按钮状态。 */
function isInstalling(repo: MarketRepo): boolean {
  return installingRepo.value === repo.fullName
}

onMounted(() => {
  loadMarket()
})
</script>

<template>
  <div class="plugin-market">
    <NSpace style="margin-bottom: 8px;" align="center">
      <NText tag="h2" style="margin: 0;">{{ t('settings.pluginMarket.title') }}</NText>
      <NButton secondary size="small" :loading="loading" @click="loadMarket">
        {{ t('settings.pluginMarket.refresh') }}
      </NButton>
    </NSpace>
    <NText depth="3" style="display: block; margin-bottom: 16px;">
      {{ t('settings.pluginMarket.subtitle') }}
    </NText>

    <div v-if="loading && repos.length === 0" class="market-state">
      <NSpin />
    </div>
    <div v-else-if="loadError" class="market-state">
      <NText type="error">{{ loadError }}</NText>
    </div>
    <NEmpty
      v-else-if="repos.length === 0"
      :description="t('settings.pluginMarket.empty')"
      style="margin-top: 64px;"
    />

    <!-- 仓库列表 -->
    <div v-else class="repo-list">
      <NCard
        v-for="repo in repos"
        :key="repo.fullName"
        size="small"
        class="repo-card"
      >
        <div class="repo-row">
          <!-- 占位图标：方形色块 + 仓库名首字母（图标链路后续迭代） -->
          <div class="repo-icon" aria-hidden="true">{{ (repo.name[0] || '').toUpperCase() }}</div>
          <div class="repo-main">
            <a
              :href="repo.htmlUrl"
              class="repo-name"
              rel="noopener"
              @click.prevent="openRepo(repo.htmlUrl)"
            >
              {{ repo.fullName }}
            </a>
            <NText depth="3" class="repo-desc">
              {{ repo.description || t('common.notAvailable') }}
            </NText>
          </div>
          <NButton
            type="primary"
            size="small"
            :loading="isInstalling(repo)"
            :disabled="installingRepo !== null && !isInstalling(repo)"
            @click="requestInstall(repo)"
          >
            {{ t('settings.pluginMarket.install') }}
          </NButton>
        </div>
      </NCard>
    </div>

    <!-- 安装确认弹窗 -->
    <NModal
      v-model:show="showInstallConfirm"
      :title="t('settings.thirdPartyPlugins.installDialogTitle')"
      preset="card"
      style="width: 420px; max-width: calc(100vw - 48px);"
    >
      <template v-if="pendingInstall">
        <NText>{{ t('settings.pluginMarket.installConfirmContent', {
          name: pendingInstall.name,
          repo: pendingInstall.fullName,
        }) }}</NText>
        <!-- 权限风险提示：第三方插件与宿主同权限执行，UI 仅样式隔离 -->
        <NAlert
          style="margin-top: 16px;"
          type="warning"
          :title="t('settings.pluginMarket.installRiskTitle')"
        >
          {{ t('settings.pluginMarket.installRiskContent') }}
        </NAlert>
        <NSpace style="margin-top: 16px;" justify="end">
          <NButton @click="pendingInstall = null">
            {{ t('settings.thirdPartyPlugins.installConfirmNegative') }}
          </NButton>
          <NButton
            type="primary"
            :loading="installingRepo !== null"
            @click="confirmInstall(pendingInstall)"
          >
            {{ t('settings.pluginMarket.installConfirmPositive') }}
          </NButton>
        </NSpace>
      </template>
    </NModal>
  </div>
</template>

<style scoped>
.plugin-market {
  flex: 1;
  min-height: 0;
  padding: 16px;
  overflow-y: auto;
}

.market-state {
  padding: 64px 0;
  display: flex;
  justify-content: center;
  align-items: center;
}

.repo-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.repo-card :deep(.n-card__content) {
  padding: 12px 16px;
}

.repo-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.repo-icon {
  width: 36px;
  height: 36px;
  flex-shrink: 0;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 16px;
  font-weight: 600;
  color: #fff;
  background: linear-gradient(135deg, #4f8ef7, #7a5cf0);
}

.repo-main {
  flex: 1;
  min-width: 0;
}

.repo-name {
  display: inline-block;
  font-weight: 600;
  font-size: 14px;
  color: var(--text-color);
  text-decoration: none;
  cursor: pointer;
}

.repo-name:hover {
  color: var(--primary-color);
}

.repo-desc {
  font-size: 13px;
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>

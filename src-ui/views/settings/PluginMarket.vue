<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  NAlert, NButton, NCard, NEmpty, NModal, NSpace, NSpin, NTag, NText,
  useDialog, useMessage,
} from 'naive-ui'
import { open } from '@tauri-apps/plugin-shell'
import { marketList, marketGetMeta, marketInstall, marketPreviewPackage, marketDiscardPreview } from '@/bridge/commands'
import type { MarketRepo, MarketPackagePreview, MarketCardMeta } from '@/bridge/commands'
import type { BridgeError } from '@/bridge/commands'
import PluginIntroModal from '@/components/settings/PluginIntroModal.vue'
import PluginPackageInfo from '@/components/settings/PluginPackageInfo.vue'

const { t } = useI18n()
const message = useMessage()
const dialog = useDialog()

const loading = ref(false)
const repos = ref<MarketRepo[]>([])
const loadError = ref<string | null>(null)
/** 插件介绍弹窗（插件形态说明）显隐。 */
const showIntro = ref(false)
/** 正在安装的仓库（按 fullName 记录，按钮 loading + 防重入）。 */
const installingRepo = ref<string | null>(null)
/** 待确认安装的仓库。 */
const pendingInstall = ref<MarketRepo | null>(null)
/** 安装确认弹窗内 manifest 预检：加载中 / 失败信息 / 预检结果（读取成功才可点「安装」）。 */
const previewLoading = ref(false)
const previewError = ref('')
const pendingPreview = ref<MarketPackagePreview | null>(null)

/** 单仓库卡片元数据加载态（失败只影响该卡，不阻断列表）。 */
interface RepoMetaState {
  loading: boolean
  error: string
  data: MarketCardMeta | null
}
/** 按仓库全名索引的卡片元数据（tag/图标/manifest），键 = MarketRepo.fullName。 */
const repoMeta = ref<Record<string, RepoMetaState>>({})

/** 卡片展示数据：元数据未到达时回落仓库自身字段（名称/描述）。 */
const cards = computed(() =>
  repos.value.map((repo) => {
    const state = repoMeta.value[repo.fullName]
    const meta = state?.data ?? null
    const manifest = meta?.manifest ?? null
    return {
      repo,
      state: state ?? null,
      /** 图标：发布侧图标资产 → 仓库所有者头像兜底 → 空（渲染首字母占位块）。 */
      icon: meta?.icon ?? meta?.ownerAvatar ?? null,
      title: manifest?.plugin.name ?? repo.name,
      tagName: meta?.tagName ?? null,
      /** 该仓库没有任何发布版本：卡片置灰、安装按钮不可点。 */
      unreleased: !!meta && meta.tagName === null,
      description: manifest?.plugin.description || repo.description,
      author: manifest?.plugin.author ?? '',
      provides: manifest?.components.provides ?? [],
      /** 元数据附件存在但不可用（如清单与该宿主不兼容）时的原因。 */
      metadataError: meta?.metadataError ?? '',
    }
  }),
)

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

/** 点击「安装」：先弹确认（下载并安装仓库最新发布）。 */
function requestInstall(repo: MarketRepo) {
  pendingInstall.value = repo
  pendingPreview.value = null
  previewError.value = ''
  void loadMarketPreview(repo.fullName)
}

/** 预检仓库最新发布插件包 manifest：下载一次并暂存，成功后展示供用户判断。 */
async function loadMarketPreview(fullName: string) {
  previewLoading.value = true
  previewError.value = ''
  pendingPreview.value = null
  try {
    const preview = await marketPreviewPackage(fullName)
    // 预检期间用户已关闭弹窗/切换目标：暂存包无人消费，立即释放
    if (pendingInstall.value?.fullName !== fullName) {
      void marketDiscardPreview(preview.assetName).catch(() => {})
      return
    }
    pendingPreview.value = preview
  } catch (e) {
    if (pendingInstall.value?.fullName !== fullName) return
    previewError.value = errorText(e)
  } finally {
    // loading 只归当前弹窗：目标匹配，或弹窗已关闭（无活跃目标）时清理
    if (pendingInstall.value?.fullName === fullName || !pendingInstall.value) {
      previewLoading.value = false
    }
  }
}

/** 弹窗关闭（取消/ESC/安装完成）后释放预检暂存；幂等，安装成功路径缓存已由后端删除。 */
function releasePendingPreview() {
  if (pendingPreview.value) {
    const assetName = pendingPreview.value.assetName
    pendingPreview.value = null
    previewError.value = ''
    void marketDiscardPreview(assetName).catch(() => {})
  } else {
    previewError.value = ''
  }
}

/** 拉取单仓库卡片元数据（tag + 图标 + manifest）；失败只落到该卡片的 error 上。
 *
 * 卡片提示只取 BridgeError.message（不带 traceId）：trace 是排查用的，混进卡片文案里
 * 只会干扰阅读，真机排查看日志即可。刷新时保留上一次的 data（只置 loading），
 * 否则卡片会在请求在途期间闪回首字母占位、已知无 release 的仓库按钮也会短暂可点。
 */
async function loadRepoMeta(fullName: string) {
  const previous = repoMeta.value[fullName]?.data ?? null
  repoMeta.value[fullName] = { loading: true, error: '', data: previous }
  try {
    const data = await marketGetMeta(fullName)
    repoMeta.value[fullName] = { loading: false, error: '', data }
  } catch (e) {
    repoMeta.value[fullName] = { loading: false, error: (e as BridgeError).message, data: null }
  }
}

/** 拉取市场仓库列表；列表先渲染，卡片元数据逐卡并行回填（不阻塞列表展示）。 */
async function loadMarket() {
  loading.value = true
  loadError.value = null
  try {
    repos.value = await marketList()
    for (const repo of repos.value) void loadRepoMeta(repo.fullName)
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
      <NButton secondary size="small" @click="showIntro = true">
        {{ t('settings.thirdPartyPlugins.intro') }}
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
        v-for="card in cards"
        :key="card.repo.fullName"
        size="small"
        class="repo-card"
      >
        <div class="repo-row">
          <!-- 图标：发布侧图标资产 → 仓库所有者头像 → 首字母占位（旧版本发布无元数据） -->
          <img v-if="card.icon" class="repo-icon" :src="card.icon" alt="" />
          <div v-else class="repo-icon repo-icon-fallback" aria-hidden="true">
            {{ (card.repo.name[0] || '').toUpperCase() }}
          </div>
          <div class="repo-main">
            <div class="repo-title">
              <a
                :href="card.repo.htmlUrl"
                class="repo-name"
                rel="noopener"
                @click.prevent="openRepo(card.repo.htmlUrl)"
              >
                {{ card.title }}
              </a>
              <NTag v-if="card.tagName" size="tiny" :bordered="false" type="primary">
                {{ card.tagName }}
              </NTag>
            </div>
            <NText depth="3" class="repo-desc">
              {{ card.description || t('common.notAvailable') }}
            </NText>
            <div class="repo-meta">
              <NText depth="3">{{ card.repo.fullName }}</NText>
              <NText v-if="card.author" depth="3">
                {{ t('settings.thirdPartyPlugins.colAuthor') }}: {{ card.author }}
              </NText>
              <NTag
                v-for="p in card.provides"
                :key="p"
                size="tiny"
                :bordered="false"
                type="info"
              >
                {{ p }}
              </NTag>
            </div>
            <!-- 卡片状态：加载中 / 拉取失败 / 无发布版本 / 元数据不可用 / 旧版本发布缺元数据 -->
            <NText v-if="card.state?.loading" depth="3" class="repo-state">
              {{ t('settings.pluginMarket.metaLoading') }}
            </NText>
            <NText v-else-if="card.state?.error" type="error" class="repo-state">
              {{ card.state.error }}
            </NText>
            <NText v-else-if="card.unreleased" depth="3" class="repo-state">
              {{ t('settings.pluginMarket.noRelease') }}
            </NText>
            <NText
              v-else-if="card.metadataError"
              type="warning"
              class="repo-state"
              :title="card.metadataError"
            >
              {{ t('settings.pluginMarket.metadataInvalid') }}: {{ card.metadataError }}
            </NText>
            <NText
              v-else-if="card.state?.data && !card.state.data.manifest"
              depth="3"
              class="repo-state"
            >
              {{ t('settings.pluginMarket.noMetadata') }}
            </NText>
          </div>
          <NButton
            type="primary"
            size="small"
            :loading="isInstalling(card.repo)"
            :disabled="
              (installingRepo !== null && !isInstalling(card.repo)) || card.unreleased
            "
            @click="requestInstall(card.repo)"
          >
            {{ t('settings.pluginMarket.install') }}
          </NButton>
        </div>
      </NCard>
    </div>

    <!-- 安装确认弹窗：包内 manifest 预检 + 风险提示 -->
    <NModal
      v-model:show="showInstallConfirm"
      :title="t('settings.thirdPartyPlugins.installDialogTitle')"
      preset="card"
      style="width: 520px; max-width: calc(100vw - 48px);"
      @after-leave="releasePendingPreview"
    >
      <template v-if="pendingInstall">
        <!-- 来源：仓库 + 发布 tag + 插件包附件名 -->
        <NText depth="3" style="word-break: break-all;">
          {{ pendingInstall.fullName }}
        </NText>
        <NText v-if="pendingPreview" depth="3" style="display: block; word-break: break-all;">
          {{ pendingPreview.tagName }} · {{ pendingPreview.assetName }}
        </NText>

        <div
          class="install-preview"
          :style="{ minHeight: previewLoading ? '72px' : undefined }"
        >
          <NSpin :show="previewLoading">
            <!-- 预检失败：展示原因并可重试 -->
            <div v-if="previewError">
              <NText type="error" style="word-break: break-all;">
                {{ t('settings.pluginMarket.previewFailed') }}: {{ previewError }}
              </NText>
              <NButton size="small" style="margin-top: 8px;" @click="loadMarketPreview(pendingInstall.fullName)">
                {{ t('settings.thirdPartyPlugins.retry') }}
              </NButton>
            </div>
            <!-- 包内 manifest 信息（真实插件身份，供用户判断） -->
            <PluginPackageInfo v-else-if="pendingPreview" :manifest="pendingPreview.manifest" />
          </NSpin>
        </div>

        <!-- 权限风险提示：第三方插件与宿主同权限执行，UI 仅样式隔离 -->
        <NAlert
          style="margin-top: 16px;"
          type="warning"
          :title="t('settings.pluginMarket.installRiskTitle')"
        >
          {{ t('settings.pluginMarket.installRiskContent') }}
        </NAlert>
        <NSpace style="margin-top: 16px;" justify="end">
          <NButton :disabled="installingRepo !== null" @click="pendingInstall = null">
            {{ t('settings.thirdPartyPlugins.installConfirmNegative') }}
          </NButton>
          <NButton
            type="primary"
            :loading="installingRepo !== null"
            :disabled="!pendingPreview"
            @click="confirmInstall(pendingInstall)"
          >
            {{ t('settings.pluginMarket.installConfirmPositive') }}
          </NButton>
        </NSpace>
      </template>
    </NModal>

    <!-- 插件介绍弹窗：插件来源 + 两种形态说明 -->
    <PluginIntroModal v-model:show="showIntro" />
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
  object-fit: contain;
}

/* 无图标兜底：色块 + 仓库名首字母 */
.repo-icon-fallback {
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

.repo-title {
  display: flex;
  align-items: center;
  gap: 8px;
}

/* 仓库全名 / 作者 / 能力标签：次要信息行 */
.repo-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  margin-top: 2px;
  font-size: 12px;
}

/* 状态行：元数据错误可能很长（TOML 解析诊断），最多两行、超出省略 */
.repo-state {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  margin-top: 2px;
  font-size: 12px;
  word-break: break-all;
}

/* 安装确认弹窗 manifest 预检区：超高时弹窗内滚动（同 PluginsManagement 弹窗） */
.install-preview {
  max-height: min(430px, calc(100vh - 360px));
  overflow-y: auto;
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

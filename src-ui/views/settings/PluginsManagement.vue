<script setup lang="ts">
import { h, ref, computed, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { i18n, resolveText, type Locale } from '@/i18n'
import { refreshPluginTranslations } from '@/stores/i18n-store'
import {
  NButton, NDataTable, NTag, NSpace, NText, NModal, NSwitch,
  NCode, NSpin, NEmpty, NAlert, NDescriptions, NDescriptionsItem, useMessage,
} from 'naive-ui'
import type { DataTableColumn } from 'naive-ui'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { open } from '@tauri-apps/plugin-shell'
import type { UnlistenFn } from '@tauri-apps/api/event'
import {
  pluginList, pluginReload, pluginUninstall,
  pluginInstallLocal, pluginGetLogs, pluginSetEnabled, pluginGetDetail,
  pickPluginZip, pickPluginDir,
} from '@/bridge/commands'
import type { InstalledPluginInfo, PluginDetail, BridgeError } from '@/bridge/commands'
import type { ComponentInfo } from '@/bridge/contract'
import { useConfigStore } from '@/stores/config-store'
import ComponentConfigLoader from '@/components/settings/ComponentConfigLoader.vue'

const { t } = useI18n()
const message = useMessage()
const configStore = useConfigStore()

/** 插件管理页统一行：内置与第三方插件合并展示，元数据均来自插件级数据（plugin_list）。 */
interface PluginRow {
  key: string
  kind: 'builtin' | 'third-party'
  name: string
  version: string
  author: string
  enabled: boolean
  plugin: InstalledPluginInfo
  componentIds: string[]
}

const pluginItems = ref<InstalledPluginInfo[]>([])
const loading = ref(false)

// 安装流程：拖拽 / 文件选择 → 确认弹窗 → pluginInstallLocal
const isDragging = ref(false)
const showInstall = ref(false)
const pendingPath = ref('')
const pendingFileName = ref('')
const installing = ref(false)
let unlistenDragDrop: UnlistenFn | null = null

// 配置展开
const expandedRowKeys = ref<string[]>([])

// Log viewer
const showLogs = ref(false)
const logPluginId = ref('')
const logContent = ref('')
const logLoading = ref(false)

// 详情查看：元数据（触发词）+ manifest
const showDetail = ref(false)
const detailLoading = ref(false)
const detailData = ref<PluginDetail | null>(null)
const showRawManifest = ref(false)
const rawManifestText = computed(() =>
  detailData.value?.manifest ? JSON.stringify(detailData.value.manifest, null, 2) : '',
)

/** 插件行：内置与第三方统一，元数据均来自 plugin_list（插件级），kind 由后端显式下发。 */
const rows = computed<PluginRow[]>(() =>
  pluginItems.value.map((p) => ({
    key: p.pluginId,
    kind: p.kind,
    name: resolveText(p.name),
    version: p.version,
    author: p.author,
    enabled: p.enabled,
    plugin: p,
    componentIds: p.componentIds,
  })),
)

const columns: DataTableColumn<PluginRow>[] = [
  {
    // naive-ui 展开行渲染器只从 type:'expand' 列读取（表格级 renderExpand 不生效）
    type: 'expand',
    width: 32,
    renderExpand(row) {
      const comps = row.componentIds
        .map((id) => configStore.components[id])
        .filter((c): c is ComponentInfo => !!c)
      if (comps.length === 0) {
        return h(NText, { depth: 3 }, { default: () => t('settings.thirdPartyPlugins.noConfig') })
      }
      return h('div', { style: 'display: flex; flex-direction: column; gap: 16px;' },
        comps.map((c) => h(ComponentConfigLoader, { component: c, key: c.componentId })),
      )
    },
  },
  {
    // 名称列：仅 panel 形态插件显示自定义图标；行内插件不展示
    title: () => t('settings.thirdPartyPlugins.colName'),
    key: 'name',
    render(row) {
      const icon = row.plugin.mode === 'panel' ? row.plugin.icon : null
      if (!icon) return row.name
      return h('span', { style: 'display: inline-flex; align-items: center; gap: 8px;' }, [
        h('img', {
          src: icon,
          alt: '',
          style: 'width: 20px; height: 20px; border-radius: 4px; object-fit: contain;',
        }),
        row.name,
      ])
    },
  },
  { title: () => t('settings.thirdPartyPlugins.colVersion'), key: 'version', width: 90 },
  {
    title: () => t('settings.thirdPartyPlugins.colAuthor'),
    key: 'author',
    width: 120,
    render(row) {
      // 内置插件无外部作者，标注「内置」
      return row.kind === 'builtin' ? t('settings.thirdPartyPlugins.builtin') : row.author
    },
  },
  {
    title: () => t('settings.thirdPartyPlugins.colState'),
    key: 'state',
    width: 90,
    render(row) {
      if (row.kind === 'builtin') {
        // 内置插件编译在程序内，运行状态恒为运行中；启用开关独立反映组件的 enabled 状态
        const on = row.enabled
        return h(
          NTag,
          { type: on ? 'success' : 'default' },
          { default: () => t(on ? 'settings.thirdPartyPlugins.stateEnabled' : 'settings.thirdPartyPlugins.stateDisabled') },
        )
      }
      const running = row.plugin.state === 'running'
      const color = running ? 'success' : 'error'
      const label = running ? t('settings.thirdPartyPlugins.stateRunning') : row.plugin.state
      return h(NTag, { type: color as never }, { default: () => label })
    },
  },
  {
    title: () => t('settings.thirdPartyPlugins.colEnabled'),
    key: 'enabled',
    width: 70,
    align: 'center',
    render(row) {
      return h(NSwitch, {
        value: row.enabled,
        onUpdateValue: (val: boolean) => handleToggleEnabled(row, val),
      })
    },
  },
  {
    title: () => t('settings.thirdPartyPlugins.colActions'),
    key: 'actions',
    // 实测 5 个 small 按钮 + 间距约 285px，列 padding 后 320 足够
    width: 320,
    fixed: 'right',
    render(row) {
      const buttons: ReturnType<typeof h>[] = []
      buttons.push(h(NButton, {
        size: 'small',
        onClick: () => handleViewDetail(row.plugin.pluginId),
      }, { default: () => t('settings.thirdPartyPlugins.details') }))
      if (row.componentIds.length > 0) {
        buttons.push(h(NButton, {
          size: 'small',
          onClick: () => toggleExpand(row.key),
        }, { default: () => t('settings.thirdPartyPlugins.config') }))
      }
      if (row.kind === 'third-party') {
        buttons.push(
          h(NButton, {
            size: 'small',
            onClick: () => handleViewLogs(row.plugin.pluginId),
          }, { default: () => t('settings.thirdPartyPlugins.logs') }),
          h(NButton, {
            size: 'small',
            disabled: true,
            onClick: () => handleReload(row.plugin.pluginId),
          }, { default: () => t('settings.thirdPartyPlugins.reload') }),
          h(NButton, {
            size: 'small',
            type: 'error',
            onClick: () => handleUninstall(row.plugin.pluginId),
          }, { default: () => t('settings.thirdPartyPlugins.uninstall') }),
        )
      }
      return h(NSpace, {}, { default: () => buttons })
    },
  },
]

/** 点击「配置」展开/收起对应行。 */
function toggleExpand(key: string) {
  expandedRowKeys.value = expandedRowKeys.value.includes(key) ? [] : [key]
}

/** NDataTable 展开行 keys 回调（RowKey 可能是 number，统一转 string）。 */
function onExpandedKeysChange(keys: Array<string | number>) {
  expandedRowKeys.value = keys.map(String)
}

/** 提取 BridgeError 的可读信息：message + traceId，便于排查。 */
function errorText(e: unknown): string {
  const err = e as BridgeError
  return err.message + (err.traceId ? ` (trace: ${err.traceId})` : '')
}

/** 刷新列表：插件（内置 + 第三方）+ 配置组件（安装/卸载会增删 ConfigManager 组件）。 */
async function loadPlugins() {
  loading.value = true
  try {
    const [plugins, _components] = await Promise.all([
      pluginList(),
      configStore.loadAllComponents(),
    ])
    pluginItems.value = plugins
    // 插件列表变化（安装/卸载/重载）后刷新合并翻译目录
    refreshPluginTranslations(i18n.global.locale.value as Locale)
  } catch (e) {
    message.error(t('settings.thirdPartyPlugins.loadListFailed') + ': ' + errorText(e))
  } finally {
    loading.value = false
  }
}

/** 注册 webview 级拖放监听；仅本组件挂载期间生效，卸载时移除。 */
async function setupDragDrop() {
  try {
    unlistenDragDrop = await getCurrentWebview().onDragDropEvent((event) => {
      const { type } = event.payload
      if (type === 'enter' || type === 'over') {
        isDragging.value = true
      } else if (type === 'leave') {
        isDragging.value = false
      } else if (type === 'drop') {
        isDragging.value = false
        handleDroppedPaths(event.payload.paths)
      }
    })
  } catch (e) {
    // 拖放监听失败不阻塞页面：拖拽安装降级不可用，文件选择仍可用
    console.warn('drag-drop listener failed:', e)
  }
}

/** 校验拖入路径：多文件时优先取第一个 .zip，其余忽略；单个目录直接放行（后端校验 manifest）。 */
function handleDroppedPaths(paths: string[]) {
  // 当前版本暂不支持第三方插件安装，拦截拖入；恢复安装时删除下面两行守卫即可
  message.warning(t('settings.thirdPartyPlugins.installNotSupported'))
  return

  const zipPaths = paths.filter((p) => p.toLowerCase().endsWith('.zip'))
  const chosen = zipPaths[0] ?? (paths.length === 1 ? paths[0] : null)
  if (!chosen) {
    message.error(t('settings.thirdPartyPlugins.onlyZipOrDir'))
    return
  }
  if (zipPaths.length > 1) {
    message.warning(t('settings.thirdPartyPlugins.singleFileOnly'))
  }
  requestInstall(chosen)
}

/** 弹出安装确认弹窗（展示文件名，避免误装）。 */
function requestInstall(path: string) {
  pendingPath.value = path
  pendingFileName.value = path.split(/[\\/]/).pop() ?? path
  showInstall.value = true
}

/** 弹出选择框并进入安装确认流程（.zip 文件与插件目录共用同一错误处理）。 */
async function chooseInstallSource(pick: () => Promise<string | null>) {
  try {
    const selected = await pick()
    if (selected) {
      requestInstall(selected)
    }
  } catch (e) {
    message.error(t('settings.thirdPartyPlugins.installFailed') + ': ' + errorText(e))
  }
}

/** 文件选择：仅 .zip（directory 模式忽略 filters，文件与目录须拆成两个入口）。 */
function handleChooseZip() {
  return chooseInstallSource(() => pickPluginZip(t('settings.thirdPartyPlugins.zipFilterLabel')))
}

/** 目录选择：开发期未打包的插件目录。 */
function handleChooseDir() {
  return chooseInstallSource(() => pickPluginDir(t('settings.thirdPartyPlugins.chooseDir')))
}

/** 确认安装：调用后端安装并刷新列表。 */
async function handleInstall() {
  if (!pendingPath.value) return
  installing.value = true
  try {
    await pluginInstallLocal(pendingPath.value)
    message.success(t('settings.thirdPartyPlugins.installSuccess'))
    showInstall.value = false
    pendingPath.value = ''
    await loadPlugins()
  } catch (e) {
    const err = e as BridgeError
    if (err.code === 'ALREADY_INSTALLED') {
      message.warning(t('settings.thirdPartyPlugins.alreadyInstalled'))
    } else if (err.code === 'COMPONENT_ID_COLLISION') {
      message.error(t('settings.thirdPartyPlugins.componentIdCollision') + ': ' + err.componentId)
    } else {
      message.error(t('settings.thirdPartyPlugins.installFailed') + ': ' + errorText(e))
    }
  } finally {
    installing.value = false
  }
}

/** 重载第三方插件。 */
async function handleReload(pluginId: string) {
  try {
    await pluginReload(pluginId)
    message.success(t('settings.thirdPartyPlugins.reloadSuccess'))
    await loadPlugins()
  } catch (e) {
    message.error(t('settings.thirdPartyPlugins.reloadFailed') + ': ' + errorText(e))
  }
}

/** 卸载第三方插件。 */
async function handleUninstall(pluginId: string) {
  try {
    await pluginUninstall(pluginId)
    message.success(t('settings.thirdPartyPlugins.uninstallSuccess'))
    await loadPlugins()
  } catch (e) {
    message.error(t('settings.thirdPartyPlugins.uninstallFailed') + ': ' + errorText(e))
  }
}

/** 启用/禁用：内置走配置组件开关，第三方走插件启停。 */
async function handleToggleEnabled(row: PluginRow, enabled: boolean) {
  try {
    if (row.kind === 'builtin') {
      // 内置插件组件 id 即插件 id
      await configStore.setEnabled(row.componentIds[0], enabled)
    } else {
      await pluginSetEnabled(row.plugin.pluginId, enabled)
    }
    // 行数据来自 plugin_list 快照（不随 configStore 响应式联动），本地同步 enabled 即时回显
    pluginItems.value = pluginItems.value.map((p) =>
      p.pluginId === row.plugin.pluginId ? { ...p, enabled } : p,
    )
    message.success(
      enabled
        ? t('settings.thirdPartyPlugins.toggleEnabledSuccess')
        : t('settings.thirdPartyPlugins.toggleDisabledSuccess'),
    )
  } catch (e) {
    message.error(t('settings.thirdPartyPlugins.toggleFailed') + ': ' + errorText(e))
  }
}

/** 查看插件详情：加载元数据（含触发词）+ 第三方 manifest 并弹出弹窗。 */
async function handleViewDetail(pluginId: string) {
  showDetail.value = true
  detailLoading.value = true
  detailData.value = null
  showRawManifest.value = false
  try {
    detailData.value = await pluginGetDetail(pluginId)
  } catch (e) {
    message.error(t('settings.thirdPartyPlugins.loadDetailFailed') + ': ' + errorText(e))
    showDetail.value = false
  } finally {
    detailLoading.value = false
  }
}

/** 用系统浏览器打开插件主页（webview 内 target=_blank 只会开空白窗口，须走 shell 插件）。 */
function handleOpenHomepage(url: string) {
  void open(url).catch(() => {
    message.error(t('settings.thirdPartyPlugins.openHomepageFailed'))
  })
}

/** 查看第三方插件日志。 */
async function handleViewLogs(pluginId: string) {
  logPluginId.value = pluginId
  showLogs.value = true
  logLoading.value = true
  try {
    const lines = await pluginGetLogs(pluginId, 100)
    logContent.value = lines.join('\n')
  } catch (e) {
    logContent.value = t('settings.thirdPartyPlugins.loadLogsFailed') + ': ' + errorText(e)
  } finally {
    logLoading.value = false
  }
}

onMounted(() => {
  loadPlugins()
  setupDragDrop()
})

onUnmounted(() => {
  // 离开页面后停止拦截拖放，避免在设置页其他区域误触发安装逻辑
  unlistenDragDrop?.()
})
</script>

<template>
  <div class="plugins-management">
    <!-- 当前版本暂不支持第三方插件安装：入口置灰 + 拖拽拦截，仅保留已装插件管理 -->
    <NAlert
      type="warning"
      style="margin-bottom: 16px;"
      :show-icon="true"
    >
      {{ t('settings.thirdPartyPlugins.installNotSupported') }}
    </NAlert>

    <NSpace style="margin-bottom: 16px;" align="center">
      <NText tag="h2" style="margin: 0;">{{ t('settings.thirdPartyPlugins.title') }}</NText>
      <NButton type="primary" disabled @click="handleChooseZip">
        {{ t('settings.thirdPartyPlugins.installFromFile') }}
      </NButton>
      <NButton secondary disabled @click="handleChooseDir">
        {{ t('settings.thirdPartyPlugins.installFromDir') }}
      </NButton>
    </NSpace>

    <!-- 拖拽安装区（拖放事件为 webview 级，仅本组件挂载期间监听） -->
    <div class="drop-zone" :class="{ dragging: isDragging }">
      <NText depth="3">{{ t('settings.thirdPartyPlugins.dropHint') }}</NText>
      <NButton size="small" secondary disabled @click="handleChooseZip">
        {{ t('settings.thirdPartyPlugins.chooseFile') }}
      </NButton>
    </div>

    <NDataTable
      :columns="columns"
      :data="rows"
      :loading="loading"
      :bordered="false"
      :row-key="(row: PluginRow) => row.key"
      :expanded-row-keys="expandedRowKeys"
      @update:expanded-row-keys="onExpandedKeysChange"
    >
      <template #empty>
        <NEmpty :description="t('settings.thirdPartyPlugins.empty')" />
      </template>
    </NDataTable>

    <!-- 安装确认弹窗 -->
    <NModal v-model:show="showInstall" :title="t('settings.thirdPartyPlugins.installDialogTitle')">
      <div style="padding: 24px; width: 420px;">
        <NText>{{ t('settings.thirdPartyPlugins.installConfirmContent', { name: pendingFileName }) }}</NText>
        <NText depth="3" style="display: block; margin-top: 8px; word-break: break-all;">
          {{ pendingPath }}
        </NText>
        <NSpace style="margin-top: 16px;" justify="end">
          <NButton :disabled="installing" @click="showInstall = false">
            {{ t('settings.thirdPartyPlugins.installConfirmNegative') }}
          </NButton>
          <NButton type="primary" :loading="installing" @click="handleInstall">
            {{ t('settings.thirdPartyPlugins.installConfirmPositive') }}
          </NButton>
        </NSpace>
      </div>
    </NModal>

    <!-- 日志查看弹窗 -->
    <NModal v-model:show="showLogs" :title="t('settings.thirdPartyPlugins.logsTitle')">
      <div style="padding: 24px; width: 600px; max-height: 400px; overflow: auto;">
        <NText depth="3" style="margin-bottom: 8px; display: block;">
          {{ t('settings.thirdPartyPlugins.logFor', { name: logPluginId }) }}
        </NText>
        <NSpin :show="logLoading">
          <NCode
            v-if="logContent"
            :code="logContent"
            language="text"
          />
          <NText v-else depth="3">{{ t('settings.thirdPartyPlugins.noLogs') }}</NText>
        </NSpin>
      </div>
    </NModal>

    <!-- 插件详情弹窗：元数据（含触发词）+ 第三方 manifest -->
    <NModal
      v-model:show="showDetail"
      :title="detailData ? resolveText(detailData.name) : t('settings.thirdPartyPlugins.detailTitle')"
      preset="card"
      style="width: 640px; max-width: calc(100vw - 48px);"
      :mask-closable="false"
    >
      <NSpin :show="detailLoading">
        <div class="plugin-detail-content">
          <template v-if="detailData">
            <!-- 触发词：插件如何被唤起 -->
            <div style="margin-bottom: 12px;">
              <NText depth="3" style="margin-right: 8px;">{{ t('settings.thirdPartyPlugins.fieldTriggerKeywords') }}</NText>
              <template v-if="detailData.triggerKeywords.length">
                <NTag
                  v-for="kw in detailData.triggerKeywords"
                  :key="kw"
                  type="info"
                  size="small"
                  style="margin-right: 6px;"
                >
                  {{ kw }}
                </NTag>
              </template>
              <NText v-else depth="3">{{ t('settings.thirdPartyPlugins.noTriggerKeywords') }}</NText>
            </div>

            <NDescriptions :column="2" bordered size="small" label-placement="left">
              <NDescriptionsItem :label="t('settings.thirdPartyPlugins.fieldDescription')" :span="2">
                {{ resolveText(detailData.description) || t('common.notAvailable') }}
              </NDescriptionsItem>
              <NDescriptionsItem :label="t('settings.thirdPartyPlugins.fieldPluginId')">
                {{ detailData.pluginId }}
              </NDescriptionsItem>
              <NDescriptionsItem :label="t('settings.thirdPartyPlugins.colVersion')">
                {{ detailData.manifest ? detailData.version : '' }}
              </NDescriptionsItem>
              <NDescriptionsItem :label="t('settings.thirdPartyPlugins.colAuthor')">
                {{ detailData.manifest ? detailData.author : t('settings.thirdPartyPlugins.builtin') }}
              </NDescriptionsItem>
              <NDescriptionsItem :label="t('settings.thirdPartyPlugins.fieldMinHostVersion')">
                {{ detailData.manifest?.plugin.minHostVersion ?? t('common.notAvailable') }}
              </NDescriptionsItem>
              <NDescriptionsItem :label="t('settings.thirdPartyPlugins.fieldLicense')">
                {{ detailData.manifest?.plugin.license || t('common.notAvailable') }}
              </NDescriptionsItem>
              <NDescriptionsItem :label="t('settings.thirdPartyPlugins.fieldPriority')">
                {{ detailData.priority }}
              </NDescriptionsItem>
              <NDescriptionsItem :label="t('settings.thirdPartyPlugins.fieldSupportedOs')" :span="2">
                {{ detailData.supportedOs.join(', ') || t('common.notAvailable') }}
              </NDescriptionsItem>
              <NDescriptionsItem
                v-if="detailData.kind === 'third-party'"
                :label="t('settings.thirdPartyPlugins.fieldRuntimeState')"
                :span="2"
              >
                <NTag :type="detailData.state === 'running' ? 'success' : 'error'" size="small">
                  {{ detailData.state === 'running'
                    ? t('settings.thirdPartyPlugins.stateRunning')
                    : detailData.state }}
                </NTag>
              </NDescriptionsItem>
              <NDescriptionsItem v-if="detailData.manifest" :label="t('settings.thirdPartyPlugins.fieldHomepage')" :span="2">
                <a
                  v-if="detailData.manifest.plugin.homepage"
                  :href="detailData.manifest.plugin.homepage"
                  @click.prevent="handleOpenHomepage(detailData.manifest.plugin.homepage)"
                  rel="noopener"
                >
                  {{ detailData.manifest.plugin.homepage }}
                </a>
                <template v-else>{{ t('common.notAvailable') }}</template>
              </NDescriptionsItem>
              <NDescriptionsItem v-if="detailData.manifest" :label="t('settings.thirdPartyPlugins.fieldProvides')" :span="2">
                {{ detailData.manifest.components.provides.join(', ') || t('common.notAvailable') }}
              </NDescriptionsItem>
            </NDescriptions>

            <!-- 运行信息（仅第三方插件 manifest 提供） -->
            <template v-if="detailData.manifest">
              <NText depth="3" style="display: block; margin: 16px 0 8px;">
                {{ t('settings.thirdPartyPlugins.fieldRuntime') }}
              </NText>
              <NDescriptions :column="2" bordered size="small" label-placement="left">
                <NDescriptionsItem :label="t('settings.thirdPartyPlugins.fieldCommand')" :span="2">
                  {{ detailData.manifest.runtime.command || t('common.notAvailable') }}
                </NDescriptionsItem>
                <NDescriptionsItem
                  v-if="detailData.manifest.runtime.args.length"
                  :label="t('settings.thirdPartyPlugins.fieldArgs')"
                  :span="2"
                >
                  {{ detailData.manifest.runtime.args.join(' ') }}
                </NDescriptionsItem>
                <NDescriptionsItem :label="t('settings.thirdPartyPlugins.fieldAutoRestart')">
                  {{ detailData.manifest.runtime.autoRestart
                    ? t('settings.thirdPartyPlugins.stateEnabled')
                    : t('settings.thirdPartyPlugins.stateDisabled') }}
                </NDescriptionsItem>
                <NDescriptionsItem :label="t('settings.thirdPartyPlugins.fieldStartupTimeout')">
                  {{ detailData.manifest.runtime.startupTimeout }}{{ t('common.secondsShort') }}
                </NDescriptionsItem>
                <NDescriptionsItem :label="t('settings.thirdPartyPlugins.fieldMaxRestart')">
                  {{ detailData.manifest.runtime.maxRestart }}
                </NDescriptionsItem>
              </NDescriptions>
              <!-- 原始 manifest（开发者友好，可折叠） -->
              <NButton quaternary size="small" style="margin-top: 12px;" @click="showRawManifest = !showRawManifest">
                {{ t('settings.thirdPartyPlugins.rawManifest') }}
              </NButton>
              <NCode v-if="showRawManifest" :code="rawManifestText" language="json" />
            </template>
          </template>
        </div>
      </NSpin>
    </NModal>
  </div>
</template>

<style scoped>
.plugins-management {
  flex: 1;
  min-height: 0;
  padding: 16px;
  overflow-y: auto;
}

.drop-zone {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 24px;
  margin-bottom: 16px;
  border: 1px dashed var(--border-color);
  border-radius: 8px;
  transition: border-color 0.2s, background-color 0.2s;
}

.drop-zone.dragging {
  border-color: var(--primary-color);
  background-color: var(--primary-color-alpha);
}
.plugin-detail-content {
  box-sizing: border-box;
  max-height: min(480px, calc(100vh - 160px));
  overflow-y: auto;
}
</style>

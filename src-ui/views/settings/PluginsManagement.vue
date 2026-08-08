<script setup lang="ts">
import { h, ref, computed, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  NButton, NDataTable, NTag, NSpace, NText, NModal, NSwitch, NIcon, NTooltip,
  NCode, NSpin, NEmpty, useMessage,
} from 'naive-ui'
import type { DataTableColumn } from 'naive-ui'
import { CircleHelp } from 'lucide-vue-next'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import type { UnlistenFn } from '@tauri-apps/api/event'
import {
  pluginList, pluginReload, pluginUninstall,
  pluginInstallLocal, pluginGetLogs, pluginSetEnabled, pickPluginZip, pickPluginDir,
} from '@/bridge/commands'
import type { InstalledPluginInfo, BridgeError } from '@/bridge/commands'
import type { ComponentInfo } from '@/bridge/contract'
import { useConfigStore } from '@/stores/config-store'
import ComponentConfigLoader from '@/components/settings/ComponentConfigLoader.vue'

const { t } = useI18n()
const message = useMessage()
const configStore = useConfigStore()

/** 插件管理页统一行：内置插件（组件级）与第三方插件（插件级）合并展示。 */
interface PluginRow {
  key: string
  kind: 'builtin' | 'third-party'
  name: string
  version: string
  author: string
  enabled: boolean
  component: ComponentInfo | null
  plugin: InstalledPluginInfo | null
  componentIds: string[]
}

const thirdPartyPlugins = ref<InstalledPluginInfo[]>([])
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

/** 第三方插件组件 id 集合，用于把配置组件从内置 Plugin 组件中排除。 */
const thirdPartyComponentIds = computed(() => {
  const ids = new Set<string>()
  for (const p of thirdPartyPlugins.value) {
    for (const id of p.componentIds) ids.add(id)
  }
  return ids
})

/** 内置插件行（组件级，componentType === 'Plugin' 且不属于任何第三方插件）。 */
const builtinRows = computed<PluginRow[]>(() =>
  Object.values(configStore.components)
    .filter((c) => c.componentType === 'Plugin' && !thirdPartyComponentIds.value.has(c.componentId))
    .map((c) => ({
      key: `builtin:${c.componentId}`,
      kind: 'builtin' as const,
      name: c.componentName,
      version: '—',
      author: '—',
      enabled: c.enabled,
      component: c,
      plugin: null,
      componentIds: [c.componentId],
    })),
)

/** 第三方插件行（插件级）。 */
const thirdPartyRows = computed<PluginRow[]>(() =>
  thirdPartyPlugins.value.map((p) => ({
    key: `third:${p.pluginId}`,
    kind: 'third-party' as const,
    name: p.name,
    version: p.version,
    author: p.author,
    enabled: p.enabled,
    component: null,
    plugin: p,
    componentIds: p.componentIds,
  })),
)

const rows = computed(() => [...builtinRows.value, ...thirdPartyRows.value])

const columns: DataTableColumn<PluginRow>[] = [
  {
    // naive-ui 展开行渲染器只从 type:'expand' 列读取（表格级 renderExpand 不生效）
    type: 'expand',
    width: 32,
    renderExpand(row) {
      if (row.kind === 'builtin' && row.component) {
        return h(ComponentConfigLoader, { component: row.component })
      }
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
  { title: () => t('settings.thirdPartyPlugins.colName'), key: 'name' },
  { title: () => t('settings.thirdPartyPlugins.colVersion'), key: 'version', width: 90, align: 'right' },
  { title: () => t('settings.thirdPartyPlugins.colAuthor'), key: 'author', width: 120, align: 'right' },
  {
    title: () =>
      h(
        'div',
        { style: 'display: flex; align-items: center; justify-content: flex-end; gap: 4px;' },
        [
          t('settings.thirdPartyPlugins.colState'),
          h(NTooltip, { placement: 'top' }, {
            trigger: () =>
              h(NIcon, { size: 14, style: 'cursor: help;' }, { default: () => h(CircleHelp) }),
            default: () => t('settings.thirdPartyPlugins.stateHint'),
          }),
        ],
      ),
    key: 'state',
    width: 90,
    align: 'right',
    render(row) {
      if (row.kind === 'builtin') {
        // 内置插件编译在程序内，运行状态恒为运行中；启用开关独立反映组件的 enabled 状态
        const on = row.component?.enabled ?? true
        return h(
          NTag,
          { type: on ? 'success' : 'default' },
          { default: () => t(on ? 'settings.thirdPartyPlugins.stateEnabled' : 'settings.thirdPartyPlugins.stateDisabled') },
        )
      }
      const running = row.plugin!.state.includes('Running')
      const color = running ? 'success' : 'error'
      const label = running ? t('settings.thirdPartyPlugins.stateRunning') : row.plugin!.state
      return h(NTag, { type: color as never }, { default: () => label })
    },
  },
  {
    title: () => t('settings.thirdPartyPlugins.colEnabled'),
    key: 'enabled',
    width: 70,
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
    // 实测 4 个 small 按钮 + 间距约 228px，列 padding 后 260 足够
    width: 120,
    fixed: 'right',
    render(row) {
      const buttons: ReturnType<typeof h>[] = []
      if (row.kind === 'builtin' || row.componentIds.length > 0) {
        buttons.push(h(NButton, {
          size: 'small',
          onClick: () => toggleExpand(row.key),
        }, { default: () => t('settings.thirdPartyPlugins.config') }))
      }
      if (row.kind === 'third-party') {
        buttons.push(
          h(NButton, {
            size: 'small',
            onClick: () => handleViewLogs(row.plugin!.pluginId),
          }, { default: () => t('settings.thirdPartyPlugins.logs') }),
          h(NButton, {
            size: 'small',
            onClick: () => handleReload(row.plugin!.pluginId),
          }, { default: () => t('settings.thirdPartyPlugins.reload') }),
          h(NButton, {
            size: 'small',
            type: 'error',
            onClick: () => handleUninstall(row.plugin!.pluginId),
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

/** 刷新列表：第三方插件 + 配置组件（安装/卸载会增删 ConfigManager 组件）。 */
async function loadPlugins() {
  loading.value = true
  try {
    const [plugins, _components] = await Promise.all([
      pluginList(),
      configStore.loadAllComponents(),
    ])
    thirdPartyPlugins.value = plugins
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
    if (row.kind === 'builtin' && row.component) {
      await configStore.setEnabled(row.component.componentId, enabled)
    } else if (row.plugin) {
      await pluginSetEnabled(row.plugin.pluginId, enabled)
    }
    message.success(
      enabled
        ? t('settings.thirdPartyPlugins.toggleEnabledSuccess')
        : t('settings.thirdPartyPlugins.toggleDisabledSuccess'),
    )
    if (row.kind === 'third-party') {
      // plugin_list 按启用状态过滤，禁用后行会消失
      await loadPlugins()
    }
  } catch (e) {
    message.error(t('settings.thirdPartyPlugins.toggleFailed') + ': ' + errorText(e))
  }
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
    <NSpace style="margin-bottom: 16px;" align="center">
      <NText tag="h2" style="margin: 0;">{{ t('settings.thirdPartyPlugins.title') }}</NText>
      <NButton type="primary" @click="handleChooseZip">
        {{ t('settings.thirdPartyPlugins.installFromFile') }}
      </NButton>
      <NButton secondary @click="handleChooseDir">
        {{ t('settings.thirdPartyPlugins.installFromDir') }}
      </NButton>
    </NSpace>

    <!-- 拖拽安装区（拖放事件为 webview 级，仅本组件挂载期间监听） -->
    <div class="drop-zone" :class="{ dragging: isDragging }">
      <NText depth="3">{{ t('settings.thirdPartyPlugins.dropHint') }}</NText>
      <NButton size="small" secondary @click="handleChooseZip">
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
</style>

<script setup lang="ts">
import { h, ref, onMounted } from 'vue'
import {
  NButton, NDataTable, NTag, NSpace, NText, NModal, NInput,
  NCode, NSpin, useMessage,
} from 'naive-ui'
import type { DataTableColumn } from 'naive-ui'
import {
  pluginList, pluginReload, pluginUninstall,
  pluginInstallLocal, pluginGetLogs, pluginSetEnabled,
} from '@/bridge/commands'
import type { InstalledPluginInfo } from '@/bridge/commands'

const message = useMessage()
const plugins = ref<InstalledPluginInfo[]>([])
const loading = ref(false)

// Install dialog
const showInstall = ref(false)
const installPath = ref('')

// Log viewer
const showLogs = ref(false)
const logPluginId = ref('')
const logContent = ref('')
const logLoading = ref(false)

const columns: DataTableColumn<InstalledPluginInfo>[] = [
  { title: '名称', key: 'name' },
  { title: '版本', key: 'version', width: 80 },
  { title: '作者', key: 'author', width: 120 },
  {
    title: '状态',
    key: 'state',
    width: 90,
    render(row) {
      const running = row.state.includes('Running')
      const color = running ? 'success' : 'error'
      const label = running ? '运行中' : row.state
      return h(NTag, { type: color as never }, { default: () => label })
    },
  },
  {
    title: '启用',
    key: 'enabled',
    width: 70,
    render(row) {
      const type = row.enabled ? 'success' : 'default'
      const label = row.enabled ? '已启用' : '已禁用'
      return h(NTag, { type: type as never }, { default: () => label })
    },
  },
  {
    title: '操作',
    key: 'actions',
    width: 340,
    render(row) {
      return h(NSpace, {}, {
        default: () => [
          h(NButton, {
            size: 'small',
            onClick: () => handleToggleEnabled(row.pluginId, !row.enabled),
          }, { default: () => row.enabled ? '禁用' : '启用' }),
          h(NButton, {
            size: 'small',
            onClick: () => handleViewLogs(row.pluginId),
          }, { default: () => '日志' }),
          h(NButton, {
            size: 'small',
            onClick: () => handleReload(row.pluginId),
          }, { default: () => '重载' }),
          h(NButton, {
            size: 'small',
            type: 'error',
            onClick: () => handleUninstall(row.pluginId),
          }, { default: () => '卸载' }),
        ],
      })
    },
  },
]

async function loadPlugins() {
  loading.value = true
  try {
    plugins.value = await pluginList()
  } catch (e) {
    message.error('加载插件列表失败: ' + String(e))
  } finally {
    loading.value = false
  }
}

async function handleReload(pluginId: string) {
  try {
    await pluginReload(pluginId)
    message.success('插件已重新加载')
    await loadPlugins()
  } catch (e) {
    message.error('重载失败: ' + String(e))
  }
}

async function handleUninstall(pluginId: string) {
  try {
    await pluginUninstall(pluginId)
    message.success('插件已卸载')
    await loadPlugins()
  } catch (e) {
    message.error('卸载失败: ' + String(e))
  }
}

async function handleInstall() {
  if (!installPath.value.trim()) {
    message.warning('请输入文件路径')
    return
  }
  try {
    await pluginInstallLocal(installPath.value)
    message.success('插件安装成功')
    showInstall.value = false
    installPath.value = ''
    await loadPlugins()
  } catch (e) {
    message.error('安装失败: ' + String(e))
  }
}

async function handleToggleEnabled(pluginId: string, enabled: boolean) {
  try {
    await pluginSetEnabled(pluginId, enabled)
    message.success(enabled ? '插件已启用' : '插件已禁用')
    await loadPlugins()
  } catch (e) {
    message.error('切换状态失败: ' + String(e))
  }
}

async function handleViewLogs(pluginId: string) {
  logPluginId.value = pluginId
  showLogs.value = true
  logLoading.value = true
  try {
    const lines = await pluginGetLogs(pluginId, 100)
    logContent.value = lines.join('\n')
  } catch (e) {
    logContent.value = '加载日志失败: ' + String(e)
  } finally {
    logLoading.value = false
  }
}

onMounted(loadPlugins)
</script>

<template>
  <div class="plugins-management">
    <NSpace style="margin-bottom: 16px;">
      <NText tag="h2" style="margin: 0;">第三方插件管理</NText>
      <NButton type="primary" @click="showInstall = true">
        从本地文件安装
      </NButton>
    </NSpace>

    <NDataTable
      :columns="columns"
      :data="plugins"
      :loading="loading"
      :bordered="false"
    />

    <!-- Install Dialog -->
    <NModal v-model:show="showInstall" title="安装插件">
      <div style="padding: 24px; width: 420px;">
        <NText>输入插件 .zip 文件路径：</NText>
        <NInput
          v-model:value="installPath"
          placeholder="C:\Downloads\my-plugin.zip"
          style="margin-top: 12px;"
        />
        <NSpace style="margin-top: 16px;" justify="end">
          <NButton @click="showInstall = false">取消</NButton>
          <NButton type="primary" @click="handleInstall">安装</NButton>
        </NSpace>
      </div>
    </NModal>

    <!-- Log Viewer -->
    <NModal v-model:show="showLogs" title="插件日志">
      <div style="padding: 24px; width: 600px; max-height: 400px; overflow: auto;">
        <NText depth="3" style="margin-bottom: 8px; display: block;">
          插件: {{ logPluginId }}
        </NText>
        <NSpin :show="logLoading">
          <NCode
            v-if="logContent"
            :code="logContent"
            language="text"
          />
          <NText v-else depth="3">暂无日志</NText>
        </NSpin>
      </div>
    </NModal>
  </div>
</template>

<style scoped>
.plugins-management {
  padding: 16px;
}
</style>

<template>
  <div class="settings-sidebar">
    <div class="sidebar-search">
      <n-popover
        trigger="manual"
        placement="bottom-start"
        :show="panelOpen"
        :show-arrow="false"
        :width="380"
        :content-style="{ padding: '0' }"
        @clickoutside="closePanel"
      >
        <template #trigger>
          <div class="search-trigger" @click="openPanel">
            <n-input
              v-model:value="query"
              size="small"
              clearable
              :placeholder="t('settings.searchBar.placeholder')"
              @focus="openPanel"
              @keydown.enter.prevent="selectFirst"
              @keydown.esc.prevent="closePanel"
            >
              <template #prefix>
                <n-icon><Search /></n-icon>
              </template>
            </n-input>
          </div>
        </template>

        <div class="search-panel">
          <div class="search-results">
            <div v-if="searching && !hits.length" class="search-loading">
              <n-spin :size="16" />
            </div>
            <button
              v-for="hit in hits"
              :key="`${hit.componentId}/${hit.fieldKey}`"
              type="button"
              class="search-hit"
              @click="selectHit(hit)"
            >
              <span class="search-hit-label">{{ hit.label }}</span>
              <span class="search-hit-path">{{ hit.path }}</span>
            </button>
            <div v-if="!searching && !hits.length" class="search-empty">
              {{ t('settings.searchBar.empty') }}
            </div>
          </div>

          <button type="button" class="search-ask" @click="askAi">
            <span class="search-ask-title">
              <n-icon><Sparkles /></n-icon>
              {{ t('settings.searchBar.askAi') }}
            </span>
            <span class="search-ask-hint">
              {{ t('settings.searchBar.askAiHint', { query: query.trim() }) }}
            </span>
          </button>
        </div>
      </n-popover>
    </div>

    <div class="sidebar-menu">
      <n-menu
        :options="menuOptions"
        :value="selectedId"
        @update:value="onSelect"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue'
import { NIcon, NInput, NMenu, NPopover, NSpin } from 'naive-ui'
import type { MenuOption } from 'naive-ui'
import { Search, Sparkles } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'
import { helpSearch } from '../../bridge/commands'
import type { HelpHit, HelpNavigation } from '../../bridge/contract'
import { useConfigStore } from '../../stores/config-store'
import { resolveText } from '../../i18n'
import { formatErrorMessage } from '../../utils/errors'
import type { SidebarCategory } from '../../utils/settingsSidebar'
import { renderSidebarIcon } from './sidebarIcons'

/** 检索输入防抖（毫秒）。 */
const SEARCH_DEBOUNCE_MS = 150

/** 命中项：宿主返回的字段信息 + 完整点击路径（分类 › [类型标签页] › 组件 › 分组）。 */
interface SettingsSearchHit extends HelpHit {
  path: string
}

const props = defineProps<{
  sidebarItems: SidebarCategory[]
  selectedId: string | null
  /** 组件 id → 完整设置页路径（与问 AI 导航共用同一份投影）。 */
  navigation: HelpNavigation[]
}>()

const emit = defineEmits<{
  (e: 'select', key: string): void
  (e: 'navigate', componentId: string, fieldKey: string): void
  (e: 'ask-ai', query: string): void
}>()

const { t } = useI18n()
const configStore = useConfigStore()

const query = ref('')
const panelOpen = ref(false)
const searching = ref(false)
const hits = ref<SettingsSearchHit[]>([])
let searchTimer: ReturnType<typeof setTimeout> | null = null
/** 检索请求序号：丢弃过期响应。 */
let searchSeq = 0

/** 侧栏分类可达的组件：只展示能跳转过去的结果（插件页等分类内的组件不在此列）。 */
const reachableComponentIds = computed(() => {
  const ids = new Set<string>()
  for (const item of props.sidebarItems) {
    for (const component of item.components ?? []) {
      ids.add(component.componentId)
    }
  }
  return ids
})

/** 组件 id → 完整点击链：复用问答页导航，保证路径与界面实际渲染形态一致。 */
const navigationPathByComponent = computed(() => {
  const paths = new Map<string, string>()
  for (const entry of props.navigation) {
    paths.set(entry.componentId, entry.path)
  }
  return paths
})

const menuOptions = computed<MenuOption[]>(() => {
  return props.sidebarItems.map(item => ({
    label: item.label,
    key: item.key,
    icon: renderSidebarIcon(item.icon)
  }))
})

/** 组件显示名：优先取组件表，缺失时退回组件 id。 */
function componentName(componentId: string): string {
  const raw = configStore.components[componentId]?.componentName
  return raw ? resolveText(raw) : componentId
}

/**
 * 命中项的完整路径：分类 › [管道类型标签页] › 组件 › 字段分组。
 * 分组为空时止于组件；同名字段靠分组区分（如浅色/深色「强调色」）。
 */
function hitPath(hit: HelpHit): string {
  const componentPath =
    navigationPathByComponent.value.get(hit.componentId) ?? componentName(hit.componentId)
  return hit.group ? `${componentPath} › ${hit.group}` : componentPath
}

/** 执行检索：宿主返回已按相关度排序的命中，前端只过滤不可跳转项并补组件名。 */
async function runSearch(value: string): Promise<void> {
  const seq = ++searchSeq
  searching.value = true
  try {
    const result = await helpSearch(value)
    if (seq !== searchSeq) return
    hits.value = result
      .filter((hit) => reachableComponentIds.value.has(hit.componentId))
      .map((hit) => ({ ...hit, path: hitPath(hit) }))
  } catch (error) {
    if (seq === searchSeq) hits.value = []
    console.warn(`[settings-search] 设置检索失败: ${formatErrorMessage(error)}`)
  } finally {
    if (seq === searchSeq) searching.value = false
  }
}

/** 打开结果浮层；查询为空时不展示。 */
function openPanel(): void {
  if (query.value.trim()) panelOpen.value = true
}

/** 关闭结果浮层（不清空查询，便于再次查看）。 */
function closePanel(): void {
  panelOpen.value = false
}

/** 选中命中项：交给父级切分类并定位字段。 */
function selectHit(hit: SettingsSearchHit): void {
  closePanel()
  emit('navigate', hit.componentId, hit.fieldKey)
}

/** 回车直达第一条命中。 */
function selectFirst(): void {
  const first = hits.value[0]
  if (first) selectHit(first)
}

/** 把当前查询交给父级，扩写成问题后转到问 AI 页。 */
function askAi(): void {
  const value = query.value.trim()
  if (!value) return
  closePanel()
  emit('ask-ai', value)
}

function onSelect(key: string): void {
  closePanel()
  emit('select', key)
}

watch(query, (value) => {
  if (searchTimer) clearTimeout(searchTimer)
  const trimmed = value.trim()
  if (!trimmed) {
    hits.value = []
    searching.value = false
    closePanel()
    return
  }
  panelOpen.value = true
  searching.value = true
  searchTimer = setTimeout(() => {
    searchTimer = null
    void runSearch(trimmed)
  }, SEARCH_DEBOUNCE_MS)
})

onUnmounted(() => {
  if (searchTimer) clearTimeout(searchTimer)
})
</script>

<style scoped>
.settings-sidebar {
  width: 200px;
  flex-shrink: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border-right: 1px solid var(--border-color);
}

.sidebar-search {
  flex-shrink: 0;
  padding: 8px 12px 4px;
}

.sidebar-menu {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 4px 0 8px;
}

.search-panel {
  display: flex;
  flex-direction: column;
  max-height: 380px;
}

.search-results {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 4px;
}

.search-loading {
  display: flex;
  justify-content: center;
  padding: 16px;
}

.search-hit {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 2px;
  width: 100%;
  padding: 8px 10px;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-color);
  font-family: inherit;
  text-align: left;
  cursor: pointer;
}

.search-hit:hover {
  background: var(--hover-color);
}

.search-hit-label {
  font-size: var(--font-size-md);
}

.search-hit-path {
  color: var(--text-color-secondary);
  font-size: var(--font-size-sm);
  line-height: 1.4;
  white-space: normal;
  overflow-wrap: anywhere;
}

.search-empty {
  padding: 16px 12px;
  color: var(--text-color-secondary);
  font-size: var(--font-size-sm);
  text-align: center;
}

.search-ask {
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  width: 100%;
  padding: 10px 12px;
  border: 0;
  border-top: 1px solid var(--border-color);
  background: transparent;
  font-family: inherit;
  text-align: left;
  cursor: pointer;
}

.search-ask:hover {
  background: var(--hover-color);
}

.search-ask-title {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--primary-color);
  font-size: var(--font-size-md);
}

.search-ask-hint {
  color: var(--text-color-secondary);
  font-size: var(--font-size-sm);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>

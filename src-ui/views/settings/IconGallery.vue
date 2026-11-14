<template>
  <div class="icon-gallery">
    <NTabs
      class="icon-gallery-tabs"
      :value="activeTab"
      type="line"
      @update:value="onTabChange"
    >
      <NTabPane
        name="gallery"
        :tab="t('settings.iconGalleryPage.galleryTab')"
        display-directive="show"
      >
        <div class="icon-gallery-pane">
          <div class="icon-gallery-toolbar">
            <NButton type="primary" :loading="retryingAll" :disabled="loading" @click="handleRetryAll">
              {{ t('settings.iconGalleryPage.retryAll') }}
            </NButton>
            <NText v-if="missingCount > 0" depth="3" class="icon-gallery-filling">
              {{ t('settings.iconGalleryPage.filling', { count: missingCount }) }}
            </NText>
          </div>

          <div class="icon-gallery-list">
            <NSpin :show="loading">
              <NList v-if="entries.length > 0" bordered>
                <NListItem v-for="entry in entries" :key="entry.id">
                  <div class="icon-gallery-row">
                    <img v-if="entry.icon" class="icon-gallery-avatar" :src="entry.icon" alt="" />
                    <NIcon
                      v-else
                      class="icon-gallery-avatar icon-gallery-avatar-placeholder"
                      :size="28"
                    >
                      <Globe />
                    </NIcon>
                    <div class="icon-gallery-meta">
                      <div class="icon-gallery-name">{{ entry.name }}</div>
                      <div class="icon-gallery-source">
                        {{ entry.source || t('settings.iconGalleryPage.embeddedIcon') }}
                      </div>
                    </div>
                  </div>
                </NListItem>
              </NList>
              <NEmpty
                v-else-if="!loading"
                :description="t('settings.iconGalleryPage.empty')"
              />
            </NSpin>
          </div>
        </div>
      </NTabPane>

      <NTabPane
        v-for="component in components"
        :key="component.componentId"
        :name="component.componentId"
        :tab="resolveText(component.componentName)"
        display-directive="show"
      >
        <div class="icon-gallery-pane icon-gallery-config">
          <ComponentConfigLoader :component="component" />
        </div>
      </NTabPane>
    </NTabs>
  </div>
</template>

<script setup lang="ts">
import { computed, inject, onMounted, onUnmounted, ref, watch } from 'vue'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { useI18n } from 'vue-i18n'
import {
  NButton,
  NEmpty,
  NIcon,
  NList,
  NListItem,
  NTabPane,
  NTabs,
  NSpin,
  NText,
  useMessage,
} from 'naive-ui'
import { Globe } from 'lucide-vue-next'
import ComponentConfigLoader from '@/components/settings/ComponentConfigLoader.vue'
import type { ComponentInfo } from '@/bridge/contract'
import { iconGalleryList, iconGalleryRetryAll } from '@/bridge/commands'
import type { IconGalleryEntry, IconGalleryList } from '@/bridge/commands'
import { onIconUpdated } from '@/bridge/events'
import { resolveText } from '@/i18n'
import { FIELD_REVEAL_KEY, type FieldRevealTarget } from '@/utils/fieldReveal'
import { UNSAVED_CHANGES_KEY } from '@/utils/unsavedChanges'
import { formatErrorMessage } from '@/utils/errors'

/** 图标页承载的组件：总览页签固定，其余组件各占一个页签（当前只有图标覆盖）。 */
const props = defineProps<{ components: ComponentInfo[] }>()

const { t } = useI18n()
const message = useMessage()
/** 设置助手跳转器：目标为图标覆盖组件时切到对应页签。 */
const reveal = inject(FIELD_REVEAL_KEY, null)
const unsavedChanges = inject(UNSAVED_CHANGES_KEY, null)

/** 当前页签：gallery 为全部图标，其余为组件 id。 */
const activeTab = ref('gallery')

/** 展示列表：候选缓存中的全部图标（后端最多返回 500 条）。 */
const entries = ref<IconGalleryEntry[]>([])
/** 列表对应的候选世代；后台回填事件按世代过滤。 */
const generation = ref(0)
/** 列表加载完成前先缓存回填事件，加载后统一应用，避免早到的图标被丢弃。 */
const pendingIcons = new Map<number, string>()
/** 列表是否已加载（决定回填事件是直接应用还是先缓存）。 */
let loaded = false
/** `bridge-icon-updated` 监听句柄（卸载时注销）。 */
let unlisten: UnlistenFn | null = null

/** 首次加载中：只读缓存，缺失图标由后端后台补齐。 */
const loading = ref(false)
/** 整体重试中。 */
const retryingAll = ref(false)
/** 仍在等待后台补齐的图标数（为 0 时隐藏进度提示）。 */
const missingCount = computed(() => entries.value.filter((entry) => !entry.icon).length)

/** 应用一页列表（同时更新世代）。 */
function apply(list: IconGalleryList) {
  generation.value = list.generation
  entries.value = list.entries
}

/** 按候选 id 就地替换图标。 */
function patchIcon(id: number, icon: string) {
  const index = entries.value.findIndex((entry) => entry.id === id)
  if (index >= 0) entries.value[index].icon = icon
}

/** 加载列表；失败时提示并保留当前内容。 */
async function load() {
  loading.value = true
  try {
    apply(await iconGalleryList())
  } catch (e) {
    message.error(formatErrorMessage(e))
  } finally {
    loading.value = false
  }
}

/** 整体重试：后端清空失败记录并重新排队补齐未缓存图标，返回当前列表。 */
async function handleRetryAll() {
  retryingAll.value = true
  try {
    apply(await iconGalleryRetryAll())
  } catch (e) {
    message.error(formatErrorMessage(e))
  } finally {
    retryingAll.value = false
  }
}

/** 切换页签：存在未保存修改时先确认丢弃，取消则留在当前页签。 */
async function onTabChange(value: string | number): Promise<void> {
  const next = String(value)
  if (next === activeTab.value) return
  if (unsavedChanges && !await unsavedChanges.confirmDiscard()) return
  activeTab.value = next
}

/** 设置助手跳转：目标组件在本页时切到它的页签（跳转不弹丢弃确认）。 */
function focusTargetTab(target: FieldRevealTarget | null): void {
  if (target && props.components.some((component) => component.componentId === target.componentId)) {
    activeTab.value = target.componentId
  }
}

/** 当前待揭示位置（无提供方时为 null）。 */
function revealTarget(): FieldRevealTarget | null {
  return reveal?.target.value ?? null
}

watch(revealTarget, focusTargetTab)

onMounted(async () => {
  // 从问答页跳转时本组件是刚挂载的：watch 看不到已发生的变更，需在挂载时补一次
  focusTargetTab(reveal?.target.value ?? null)
  unlisten = await onIconUpdated((payload) => {
    if (!loaded) {
      pendingIcons.set(payload.id, payload.icon)
      return
    }
    if (payload.candidateGeneration === generation.value) {
      patchIcon(payload.id, payload.icon)
    }
  })
  await load()
  loaded = true
  for (const [id, icon] of pendingIcons) {
    patchIcon(id, icon)
  }
  pendingIcons.clear()
})

onUnmounted(() => {
  unlisten?.()
  unlisten = null
})
</script>

<style scoped>
.icon-gallery {
  flex: 1;
  min-height: 0;
  padding: 16px;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
}

.icon-gallery-tabs {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.icon-gallery :deep(.n-tab-pane) {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  box-sizing: border-box;
}

.icon-gallery-pane {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.icon-gallery-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 8px;
}

.icon-gallery-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
}

.icon-gallery-list :deep(.n-list-item__main) {
  min-width: 0;
}

.icon-gallery-config {
  overflow: hidden;
  padding-top: 8px;
}

.icon-gallery-filling {
  font-size: var(--font-size-sm);
}

.icon-gallery-row {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
}

.icon-gallery-avatar {
  width: 28px;
  height: 28px;
  flex-shrink: 0;
  border-radius: 4px;
  object-fit: contain;
}

.icon-gallery-avatar-placeholder {
  color: var(--text-secondary);
}

.icon-gallery-meta {
  flex: 1;
  min-width: 0;
}

.icon-gallery-name {
  font-size: var(--font-size-base);
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.icon-gallery-source {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>

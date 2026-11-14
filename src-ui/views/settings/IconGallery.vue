<template>
  <div class="icon-gallery">
    <NSpace class="icon-gallery-toolbar" align="center">
      <NText tag="h2" class="icon-gallery-title">{{ t('settings.sidebar.iconGallery') }}</NText>
      <NButton type="primary" :loading="retryingAll" :disabled="loading" @click="handleRetryAll">
        {{ t('settings.iconGalleryPage.retryAll') }}
      </NButton>
    </NSpace>

    <div class="icon-gallery-hint">
      <NText depth="3">{{ t('settings.iconGalleryPage.customizeHint') }}</NText>
      <NButton size="small" quaternary type="primary" @click="goToIconSettings">
        {{ t('settings.iconGalleryPage.customizeAction') }}
      </NButton>
    </div>

    <NText v-if="missingCount > 0" depth="3" class="icon-gallery-filling">
      {{ t('settings.iconGalleryPage.filling', { count: missingCount }) }}
    </NText>

    <NSpin :show="loading">
      <NList v-if="entries.length > 0" bordered>
        <NListItem v-for="entry in entries" :key="entry.id">
          <div class="icon-gallery-row">
            <img v-if="entry.icon" class="icon-gallery-avatar" :src="entry.icon" alt="" />
            <NIcon v-else class="icon-gallery-avatar icon-gallery-avatar-placeholder" :size="28">
              <Globe />
            </NIcon>
            <div class="icon-gallery-meta">
              <div class="icon-gallery-name">{{ entry.name }}</div>
              <div class="icon-gallery-source">{{ entry.source }}</div>
            </div>
          </div>
        </NListItem>
      </NList>
      <NEmpty v-else-if="!loading" :description="t('settings.iconGalleryPage.empty')" />
    </NSpin>
  </div>
</template>

<script setup lang="ts">
import { computed, inject, onMounted, onUnmounted, ref } from 'vue'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { useI18n } from 'vue-i18n'
import {
  NButton,
  NEmpty,
  NIcon,
  NList,
  NListItem,
  NSpace,
  NSpin,
  NText,
  useMessage,
} from 'naive-ui'
import { Globe } from 'lucide-vue-next'
import { iconGalleryList, iconGalleryRetryAll } from '@/bridge/commands'
import type { IconGalleryEntry, IconGalleryList } from '@/bridge/commands'
import { onIconUpdated } from '@/bridge/events'
import { FIELD_REVEAL_KEY } from '@/utils/fieldReveal'
import { formatErrorMessage } from '@/utils/errors'

const { t } = useI18n()
const message = useMessage()
/** 设置页跳转器：切到图标覆盖设置所在的分类与 tab。 */
const reveal = inject(FIELD_REVEAL_KEY, null)

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

/** 跳转到图标覆盖设置页（外观分类下的 icon-override-config）。 */
function goToIconSettings() {
  reveal?.request('icon-override-config', '')
}

onMounted(async () => {
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
  overflow-y: auto;
}

.icon-gallery-toolbar {
  margin-bottom: 8px;
}

.icon-gallery-title {
  margin: 0;
}

.icon-gallery-hint {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-bottom: 12px;
  font-size: var(--font-size-sm);
}

.icon-gallery-filling {
  display: block;
  margin-bottom: 8px;
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

<template>
  <div class="smart-target-panel">
    <div class="st-head">
      <!-- data.icon 为后端解析出的 data URL（可空）；空时按 kind 渲染占位字形 -->
      <img v-if="iconSrc" class="st-icon" :src="iconSrc" alt="" />
      <n-icon v-else class="st-icon st-icon--placeholder" :size="28" color="var(--text-secondary)">
        <Folder v-if="data.kind === 'path'" />
        <Link v-else />
      </n-icon>
      <div class="st-text">
        <div class="st-title">{{ titleText }}</div>
        <div v-if="data.subtitle" class="st-subtitle">{{ data.subtitle }}</div>
      </div>
    </div>

    <div v-if="actions.length > 0" class="st-actions">
      <n-button
        v-for="action in actions"
        :key="action.id"
        size="small"
        :type="action.isDefault ? 'primary' : 'default'"
        @click="executeAction(action)"
      >
        {{ resolveText(action.label) }}
      </n-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NButton, NIcon } from 'naive-ui'
import { Folder, Link } from 'lucide-vue-next'
import type { ResultAction } from '@/bridge/contract'
import { resolveText } from '@/i18n'
import { useSearchStore } from '@/stores/search-store'

/** 后端 path-detect / url-detect 共用的面板数据形状。 */
interface SmartTargetData {
  /** 解析类别：路径 / 网址（决定占位字形与默认动作语义）。 */
  kind: 'path' | 'url'
  /** 解析后的执行目标（实际打开对象）。 */
  target: string
  /** 标题：i18n key 或字面量。 */
  title: string
  /** 展示用副标题。 */
  subtitle: string
  /** 图标 data URL（可空）。 */
  icon: string | null
}

const props = defineProps<{
  data: SmartTargetData
  actions: ResultAction[]
}>()

const searchStore = useSearchStore()

const iconSrc = computed(() => props.data?.icon ?? '')
// 标题为后端下发的 i18n key（或字面量），按 key-or-literal 约定渲染
const titleText = computed(() => {
  const t = props.data?.title ?? ''
  return t ? resolveText(t) : ''
})

// 所有面板动作统一经 bridge_confirm 委托后端执行（RULES.md 前后端职责边界）：
// 打开路径/文件夹、浏览器跳转均由后端完成，前端不做平台操作。
// candidate_id=0 为插件模式虚拟值，后端按 plugin_id 路由。
async function executeAction(action: ResultAction) {
  await searchStore.doConfirm(0, action.id)
}
</script>

<style scoped>
.smart-target-panel {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.st-head {
  display: flex;
  align-items: center;
  gap: 12px;
  background: var(--bg-secondary);
  border-radius: var(--radius-sm);
  padding: 16px;
}

.st-icon {
  width: 28px;
  height: 28px;
  flex-shrink: 0;
  object-fit: contain;
}

.st-icon--placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
}

.st-text {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.st-title {
  font-size: var(--font-size-base);
  font-weight: 600;
  color: var(--text-primary);
}

.st-subtitle {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  word-break: break-all;
}

.st-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}
</style>

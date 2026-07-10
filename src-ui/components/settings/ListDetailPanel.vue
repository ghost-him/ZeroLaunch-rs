<template>
  <div class="list-detail-panel">
    <!-- 左侧：项目列表 -->
    <div class="panel-list">
      <div class="list-header">
        <span class="list-title">{{ title }}</span>
        <span class="list-count">{{ items.length }}</span>
      </div>
      <div class="list-scroll">
        <div
          v-for="item in items"
          :key="item.componentId"
          class="list-item"
          :class="{
            'is-selected': selectedId === item.componentId,
            'is-disabled': !item.enabled,
          }"
          @click="selectItem(item.componentId)"
        >
          <div class="item-icon">
            <n-icon :size="16">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
              >
                <path
                  d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"
                />
              </svg>
            </n-icon>
          </div>
          <div class="item-info">
            <span class="item-name">{{ item.componentName }}</span>
            <span class="item-id">{{ item.componentId }}</span>
          </div>
          <div class="item-toggle" @click.stop>
            <n-switch
              :value="item.enabled"
              size="small"
              @update:value="(val: boolean) => onItemToggle(item, val)"
            />
          </div>
        </div>
        <div v-if="items.length === 0" class="list-empty">
          <n-text depth="3">{{ $t('settings.noComponents') }}</n-text>
        </div>
      </div>
    </div>

    <!-- 右侧：详情配置 -->
    <div class="panel-detail">
      <template v-if="selectedItem">
        <ComponentConfigLoader :key="selectedItem.componentId" :component="selectedItem" />
      </template>
      <div v-else class="detail-empty">
        <div class="empty-icon">
          <n-icon :size="32">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
            >
              <path
                d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"
              />
              <polyline points="14 2 14 8 20 8" />
            </svg>
          </n-icon>
        </div>
        <n-text depth="3">{{ $t('settings.selectComponent') }}</n-text>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { NSwitch, NText, NIcon, useMessage } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import type { ComponentInfo } from '../../bridge/contract'
import ComponentConfigLoader from './ComponentConfigLoader.vue'
import { useConfigStore } from '../../stores/config-store'

const props = withDefaults(
  defineProps<{
    /** 组件列表 */
    items: ComponentInfo[]
    /** 列表标题 */
    title: string
    /** 是否使用自定义切换逻辑（由父组件处理 toggle） */
    customToggle?: boolean
  }>(),
  {
    customToggle: false,
  },
)

const emit = defineEmits<{
  (e: 'toggle', componentId: string, enabled: boolean): void
}>()

const configStore = useConfigStore()
const message = useMessage()
const { t } = useI18n()
const selectedId = ref<string | null>(null)

const selectedItem = computed(() => {
  if (!selectedId.value) return null
  return props.items.find((c) => c.componentId === selectedId.value) ?? null
})

function selectItem(id: string) {
  selectedId.value = id
}

async function onItemToggle(item: ComponentInfo, val: boolean) {
  if (props.customToggle) {
    emit('toggle', item.componentId, val)
    return
  }
  try {
    await configStore.setEnabled(item.componentId, val)
  } catch (e) {
    message.error(t('error.actionFailed'))
  }
}
</script>

<style scoped>
.list-detail-panel {
  display: flex;
  gap: 0;
  height: 100%;
  min-height: 400px;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  overflow: hidden;
  background-color: var(--bg-color);
}

/* ===== 左侧列表 ===== */
.panel-list {
  width: 240px;
  flex-shrink: 0;
  border-right: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
}

.list-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
}

.list-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-color);
}

.list-count {
  font-size: 11px;
  color: var(--text-color-secondary);
  background-color: var(--bg-color);
  padding: 1px 8px;
  border-radius: 10px;
  min-width: 18px;
  text-align: center;
}

.list-scroll {
  flex: 1;
  overflow-y: auto;
  padding: 4px;
}

.list-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-radius: 6px;
  cursor: pointer;
  transition: background-color 0.15s;
  margin-bottom: 2px;
}

.list-item:hover {
  background-color: var(--hover-color);
}

.list-item.is-selected {
  background-color: var(--primary-color-alpha);
}

.list-item.is-disabled {
  opacity: 0.5;
}

.item-icon {
  width: 28px;
  height: 28px;
  border-radius: 6px;
  background-color: var(--primary-color-alpha);
  color: var(--primary-color);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.item-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.item-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-color);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.item-id {
  font-size: 11px;
  color: var(--text-color-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.item-toggle {
  flex-shrink: 0;
}

.list-empty {
  padding: 24px;
  text-align: center;
}

/* ===== 右侧详情 ===== */
.panel-detail {
  flex: 1;
  overflow-y: auto;
  min-width: 0;
}

.detail-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  gap: 12px;
  color: var(--text-color-secondary);
  padding: 32px;
  text-align: center;
}

.empty-icon {
  opacity: 0.3;
}
</style>

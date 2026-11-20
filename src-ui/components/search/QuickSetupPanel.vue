<template>
  <div ref="rootRef" class="quick-setup" tabindex="-1">
    <div class="quick-setup-header">
      <div class="quick-setup-title">{{ t('search.quickSetup.title') }}</div>
      <div class="quick-setup-desc">{{ t('search.quickSetup.description') }}</div>
    </div>

    <div v-if="loading" class="quick-setup-empty">{{ t('search.quickSetup.loading') }}</div>
    <div v-else-if="fields.length === 0" class="quick-setup-empty">
      {{ t('search.quickSetup.empty') }}
    </div>
    <div v-else class="quick-setup-fields">
      <DynamicFormField
        v-for="item in fields"
        :key="`${item.componentId}.${item.key}`"
        :field="item.config"
        :component-id="item.componentId"
        :model-value="valueOf(item)"
        @update:model-value="(val: unknown) => onValueChange(item, val)"
      />
    </div>

    <div class="quick-setup-footer">
      <span class="quick-setup-hint">{{ t('search.quickSetup.hint', { hotkey: hotkeyLabel }) }}</span>
      <div class="quick-setup-actions">
        <n-button text size="small" :disabled="loading" @click="openFullSettings">
          {{ t('search.quickSetup.openSettings') }}
        </n-button>
        <n-button size="small" type="primary" :loading="finishing" :disabled="loading" @click="onFinish">
          {{ t('search.quickSetup.done') }}
        </n-button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { nextTick, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { NButton } from 'naive-ui'
import DynamicFormField from '../settings/DynamicFormField.vue'
import { QUICK_SETUP_FIELDS, type QuickSetupFieldRef } from './quickSetupFields'
import { useSearchStore } from '../../stores/search-store'
import { useConfigStore } from '../../stores/config-store'
import { useSettings } from '../../composables/useSettings'
import { configGetSettings } from '../../bridge/commands'
import { buildFieldConfigs, type FieldConfig } from '../../utils/schemaTypes'

/** 清单项 + 宿主 schema 解析出的字段配置：控件类型完全由 schema 决定。 */
interface RenderedField extends QuickSetupFieldRef {
  config: FieldConfig
}

const store = useSearchStore()
const configStore = useConfigStore()
const { openSettings } = useSettings()
const { t } = useI18n()

const rootRef = ref<HTMLElement | null>(null)
const fields = ref<RenderedField[]>([])
/** 各组件当前设置值：写入时必须整组件回传，故保留完整快照。 */
const values = ref<Record<string, Record<string, unknown>>>({})
const loading = ref(true)
const finishing = ref(false)
/** 当前唤出热键（提示用）：读取失败回退出厂默认。 */
const hotkeyLabel = ref('Alt+Space')

/// 按清单读取 schema 与当前值：字段缺失或读取失败只跳过该项，不阻断整页。
async function load() {
  // 唤出热键提示：读取失败保留默认值，不影响设置项加载。
  try {
    const hotkey = (await configGetSettings('hotkey-config')) as Record<string, unknown> | null
    hotkeyLabel.value = ((hotkey?.open_search_bar as string) || '').trim() || 'Alt+Space'
  } catch (e) {
    console.warn('[quick-setup] 读取唤出热键失败:', e)
  }

  const loaded: RenderedField[] = []
  for (const item of QUICK_SETUP_FIELDS) {
    try {
      const schema = await configStore.getSchema(item.componentId)
      const config = buildFieldConfigs(schema.contribution).find((f) => f.key === item.key)
      if (!config) {
        console.warn(`[quick-setup] schema 缺少字段: ${item.componentId}.${item.key}`)
        continue
      }
      if (!values.value[item.componentId]) {
        const current = (await configGetSettings(item.componentId)) as Record<string, unknown> | null
        values.value = { ...values.value, [item.componentId]: { ...(current ?? {}) } }
      }
      loaded.push({ ...item, config })
    } catch (e) {
      console.warn(`[quick-setup] 读取字段失败: ${item.componentId}.${item.key}`, e)
    }
  }
  fields.value = loaded
}

/// 取字段当前展示值。
function valueOf(item: RenderedField): unknown {
  return values.value[item.componentId]?.[item.key]
}

/// 切换字段值：先乐观更新展示，再串行写回后端；失败回滚（错误已由全局处理器提示）。
async function onValueChange(item: RenderedField, value: unknown) {
  const previous = valueOf(item)
  values.value = {
    ...values.value,
    [item.componentId]: { ...values.value[item.componentId], [item.key]: value },
  }
  try {
    await store.writeQuickSetupField(item.componentId, item.key, value)
  } catch {
    values.value = {
      ...values.value,
      [item.componentId]: { ...values.value[item.componentId], [item.key]: previous },
    }
  }
}

/// 打开完整设置窗口：引导标记保持不变，下次唤起仍会显示本页。
async function openFullSettings() {
  try {
    await openSettings()
  } catch (e) {
    console.warn('[quick-setup] 打开设置窗口失败:', e)
  }
}

/// 完成引导：把页面当前值写入并落引导标记；失败保留页面供重试。
async function onFinish() {
  finishing.value = true
  try {
    await store.completeQuickSetup(values.value)
  } catch {
    // 失败原因已由全局错误处理器提示，页面保持打开
  } finally {
    finishing.value = false
  }
}

/// 宿主键盘的 Enter/Esc 请求：复用按钮的完成逻辑，保留写入失败重试与页面状态。
watch(
  () => store.quickSetupFinishRequest,
  () => {
    if (loading.value || finishing.value) return
    void onFinish()
  },
)

onMounted(async () => {
  await load()
  loading.value = false
  await nextTick()
  rootRef.value?.focus()
})
</script>

<style scoped>
.quick-setup {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 22px 28px 16px;
  outline: none;
}

.quick-setup-header {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.quick-setup-title {
  font-size: var(--font-size-lg);
  font-weight: 600;
  color: var(--text-primary);
}

.quick-setup-desc {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}

.quick-setup-empty {
  padding: 16px 0;
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}

.quick-setup-fields {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.quick-setup-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding-top: 12px;
  border-top: 1px solid var(--border-color);
}

.quick-setup-hint {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}

.quick-setup-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;
}
</style>

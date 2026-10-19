<template>
  <div class="assistant-view">
    <div class="assistant-toolbar">
      <h3>{{ t('settings.assistant.title') }}</h3>
      <n-text depth="3" class="assistant-subtitle">{{ t('settings.assistant.subtitle') }}</n-text>
      <n-button v-if="store.messages.length" size="tiny" quaternary @click="store.clear()">
        {{ t('settings.assistant.clear') }}
      </n-button>
    </div>

    <div ref="messagesRef" class="assistant-messages">
      <div v-if="!store.messages.length" class="assistant-empty">
        <n-text depth="3">{{ t('settings.assistant.empty') }}</n-text>
        <div class="assistant-samples">
          <n-button
            v-for="sample in samples"
            :key="sample"
            size="small"
            secondary
            @click="sendText(sample)"
          >
            {{ sample }}
          </n-button>
        </div>
      </div>

      <div
        v-for="(message, index) in store.messages"
        :key="index"
        class="message-row"
        :class="message.role"
      >
        <div class="message-bubble">
          <p class="message-text">{{ message.content }}</p>
          <div v-if="message.references.length" class="message-refs">
            <n-button
              v-for="reference in message.references"
              :key="`${reference.componentId}/${reference.fieldKey}`"
              size="tiny"
              secondary
              @click="jump(reference.componentId, reference.fieldKey)"
            >
              {{ referenceLabel(reference) }}
            </n-button>
          </div>
          <div v-if="message.role === 'assistant'" class="message-actions">
            <n-button size="tiny" quaternary @click="copyAnswer(message.content)">
              {{ t('settings.assistant.copy') }}
            </n-button>
          </div>
        </div>
      </div>

      <div v-if="store.asking" class="message-row assistant">
        <div class="message-bubble message-pending">
          <n-spin :size="14" />
          <span>{{ t('settings.assistant.thinking') }}</span>
        </div>
      </div>
    </div>

    <div class="assistant-composer">
      <div v-if="!modelReady" class="composer-tip">
        <n-text depth="3">{{ t('settings.assistant.noModel') }}</n-text>
        <n-button
          size="tiny"
          text
          type="primary"
          @click="jump(HELP_AGENT_CONFIG_ID, 'model_id')"
        >
          {{ t('settings.assistant.configure') }}
        </n-button>
        <n-text depth="3" class="composer-model-hint">
          {{ t('settings.assistant.modelToolHint') }}
        </n-text>
      </div>

      <!-- 输入即匹配设置项：比问一轮更快，点一下直接跳过去 -->
      <div v-if="hits.length" class="composer-hits">
        <span class="hits-label">{{ t('settings.assistant.directJump') }}</span>
        <n-button
          v-for="hit in hits"
          :key="`${hit.componentId}/${hit.fieldKey}`"
          size="tiny"
          quaternary
          @click="jump(hit.componentId, hit.fieldKey)"
        >
          {{ resolveText(hit.label) || hit.fieldKey }}
        </n-button>
      </div>

      <n-input
        v-model:value="draft"
        type="textarea"
        :autosize="{ minRows: 1, maxRows: 4 }"
        :placeholder="t('settings.assistant.placeholder')"
        @keydown.enter.exact.prevent="send"
      />
      <div class="composer-actions">
        <n-text depth="3" class="composer-hint">{{ t('settings.assistant.enterHint') }}</n-text>
        <n-button type="primary" size="small" :loading="store.asking" :disabled="!canSend" @click="send">
          {{ t('settings.assistant.send') }}
        </n-button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, inject, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { NButton, NInput, NSpin, NText, useMessage } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import { copyToClipboard, helpSearch } from '../../bridge/commands'
import type { HelpHit, HelpNavigation, HelpReference } from '../../bridge/contract'
import { useAssistantStore } from '../../stores/assistant-store'
import { useConfigStore } from '../../stores/config-store'
import { FIELD_REVEAL_KEY } from '../../utils/fieldReveal'
import { resolveText } from '../../i18n'
import { formatErrorMessage } from '../../utils/errors'

/** 设置助手配置组件 id（与宿主 HELP_AGENT_CONFIG_ID 一致）。 */
const HELP_AGENT_CONFIG_ID = 'help-agent-config'

/** 检索输入防抖（毫秒）。 */
const SEARCH_DEBOUNCE_MS = 150

const props = defineProps<{
  /** 组件的设置页点击路径（宿主写入说明书，用于逐步导航）。 */
  navigation: HelpNavigation[]
}>()

const store = useAssistantStore()
const configStore = useConfigStore()
const { t } = useI18n()
/** 消息提示（模板中的 `message` 是对话消息项，故 API 另取名）。 */
const messageApi = useMessage()
const reveal = inject(FIELD_REVEAL_KEY)
if (!reveal) {
  throw new Error('AssistantView 需置于提供 FIELD_REVEAL_KEY 的设置页内')
}

const draft = ref('')
const hits = ref<HelpHit[]>([])
const messagesRef = ref<HTMLElement | null>(null)
let searchTimer: ReturnType<typeof setTimeout> | null = null
/** 检索请求序号：丢弃过期响应。 */
let searchSeq = 0

const samples = computed(() => [
  t('settings.assistant.sample1'),
  t('settings.assistant.sample2'),
  t('settings.assistant.sample3'),
])

/**
 * 助手是否已配置模型（未配置时只做检索，不发起提问）。
 * 镜像后端 `AgentError::ModelNotConfigured` 的判定（model_id 为空）；后端判定变更时同步。
 */
const modelReady = computed(() => {
  const raw = configStore.settings[HELP_AGENT_CONFIG_ID] as { model_id?: unknown } | undefined
  return typeof raw?.model_id === 'string' && raw.model_id.trim().length > 0
})

const canSend = computed(() => draft.value.trim().length > 0 && !store.asking)

/** 引用按钮文案：有字段标签时带上组件名，组件级引用只显示组件名。 */
function referenceLabel(reference: HelpReference): string {
  const componentName = resolveText(reference.componentName)
  const fieldLabel = resolveText(reference.fieldLabel)
  return fieldLabel ? `${componentName} · ${fieldLabel}` : componentName
}

/** 跳到某个设置位置（引用与检索命中共用）。 */
function jump(componentId: string, fieldKey: string): void {
  hits.value = []
  const found = reveal!.request(componentId, fieldKey)
  if (!found) messageApi.warning(t('settings.assistant.unreachable'))
}

/** 复制一条答复文本到剪贴板（走宿主 IPC，失败给出提示）。 */
async function copyAnswer(text: string): Promise<void> {
  try {
    await copyToClipboard(text)
    messageApi.success(t('settings.assistant.copied'))
  } catch (error) {
    messageApi.error(`${t('settings.assistant.copyFailed')}：${formatErrorMessage(error)}`)
  }
}

/** 发送当前输入。 */
async function send(): Promise<void> {
  const question = draft.value.trim()
  if (!question || store.asking) return
  draft.value = ''
  hits.value = []
  await store.ask(question, props.navigation)
}

/** 用样例问题直接发送。 */
function sendText(question: string): void {
  draft.value = question
  void send()
}

/** 滚动到最新消息。 */
function scrollToBottom(): void {
  void nextTick(() => {
    const element = messagesRef.value
    if (element) element.scrollTop = element.scrollHeight
  })
}

/** 防抖检索设置项（失败静默，仅保留提问路径）。 */
function scheduleSearch(): void {
  if (searchTimer) clearTimeout(searchTimer)
  const value = draft.value.trim()
  if (!value) {
    hits.value = []
    return
  }
  searchTimer = setTimeout(async () => {
    const seq = ++searchSeq
    try {
      const result = await helpSearch(value)
      if (seq === searchSeq) hits.value = result.slice(0, 5)
    } catch (error) {
      if (seq === searchSeq) hits.value = []
      console.warn(`[assistant] 设置检索失败: ${formatErrorMessage(error)}`)
    }
  }, SEARCH_DEBOUNCE_MS)
}

watch(draft, () => scheduleSearch())
watch(() => store.messages.length, () => scrollToBottom())
watch(
  () => store.asking,
  (asking) => {
    if (asking) scrollToBottom()
  },
)

/** 助手配置按需加载：本页自行取一次，避免未访问过配置页时误判「尚未配置模型」。 */
onMounted(async () => {
  try {
    await configStore.getSettings(HELP_AGENT_CONFIG_ID)
  } catch (error) {
    console.warn(`[assistant] 助手配置读取失败: ${formatErrorMessage(error)}`)
  }
})

onUnmounted(() => {
  if (searchTimer) clearTimeout(searchTimer)
})
</script>

<style scoped>
.assistant-view {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}
.assistant-toolbar {
  display: flex;
  align-items: baseline;
  gap: 12px;
  padding: 16px 24px 8px;
  border-bottom: 1px solid var(--border-color);
}
.assistant-toolbar h3 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
}
.assistant-subtitle {
  flex: 1;
  font-size: var(--font-size-sm);
}
.assistant-messages {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 16px 24px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.assistant-empty {
  margin: auto;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  text-align: center;
}
.assistant-samples {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 8px;
  max-width: 460px;
}
.message-row {
  display: flex;
  justify-content: flex-start;
}
.message-row.user {
  justify-content: flex-end;
}
.message-bubble {
  max-width: 76%;
  padding: 10px 12px;
  border-radius: 10px;
  background: var(--bg-color-secondary);
  border: 1px solid var(--border-color);
}
.message-row.user .message-bubble {
  background: var(--primary-color);
  border-color: transparent;
  color: var(--text-on-accent);
}
.message-text {
  margin: 0;
  font-size: var(--font-size-md);
  line-height: 1.7;
  white-space: pre-wrap;
  word-break: break-word;
  /* 对话文本可选中复制（覆盖全局 body 的 user-select: none） */
  user-select: text;
  -webkit-user-select: text;
}
.message-refs {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 8px;
}
.message-actions {
  display: flex;
  justify-content: flex-end;
  margin-top: 4px;
}
.message-pending {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: var(--font-size-sm);
}
.assistant-composer {
  flex-shrink: 0;
  padding: 12px 24px 16px;
  border-top: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.composer-tip,
.composer-hits {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: var(--font-size-sm);
  flex-wrap: wrap;
}
.hits-label {
  opacity: 0.6;
}
.composer-model-hint {
  flex-basis: 100%;
}
.composer-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.composer-hint {
  font-size: var(--font-size-sm);
}
</style>

import { defineStore } from 'pinia'
import { ref } from 'vue'
import { helpAsk } from '../bridge/commands'
import { formatErrorMessage } from '../utils/errors'
import type { HelpMessage, HelpNavigation, HelpReference } from '../bridge/contract'

/** 下发给宿主的历史条数上限；宿主另有权威裁剪（≤40 条 / ≤64 * 1024 字符）。 */
const HISTORY_LIMIT = 40

/** 对话消息：角色 + 文本 + 可跳转引用。 */
export interface AssistantMessage {
  role: 'user' | 'assistant'
  content: string
  references: HelpReference[]
}

/**
 * AI 助手对话状态。
 *
 * 放在 store 而非页面内：跳到设置项再回到问答页时对话不丢。
 */
export const useAssistantStore = defineStore('assistant', () => {
  const messages = ref<AssistantMessage[]>([])
  const asking = ref(false)

  /**
   * 发送一个问题：先追加用户消息，再等待宿主答复并追加助手消息。
   * 参数：question - 问题文本；navigation - 设置页路径（由宿主写入说明书，用于逐步导航）。
   * 失败时把错误文案作为助手消息追加（全局错误处理器另有通知）。
   */
  async function ask(question: string, navigation: HelpNavigation[]): Promise<void> {
    if (asking.value) return
    // 历史在追加本次提问之前取：问题由 question 单独下发，重复带入会让同一句话出现两次
    const history: HelpMessage[] = messages.value.slice(-HISTORY_LIMIT).map((message) => ({
      role: message.role,
      content: message.content,
    }))
    const userMessage: AssistantMessage = { role: 'user', content: question, references: [] }
    messages.value = [...messages.value, userMessage]
    asking.value = true
    try {
      const answer = await helpAsk(question, navigation, history)
      const assistantMessage: AssistantMessage = {
        role: 'assistant',
        content: answer.answer,
        references: answer.references,
      }
      messages.value = [...messages.value, assistantMessage]
    } catch (error) {
      const errorMessage: AssistantMessage = {
        role: 'assistant',
        content: formatErrorMessage(error),
        references: [],
      }
      messages.value = [...messages.value, errorMessage]
    } finally {
      asking.value = false
    }
  }

  /** 清空对话。 */
  function clear(): void {
    messages.value = []
  }

  return { messages, asking, ask, clear }
})

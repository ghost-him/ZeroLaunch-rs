/**
 * 前端全局错误落盘：把未捕获异常与 Promise 拒绝经 IPC 写入后端日志。
 *
 * release 下 WebView2 控制台不可见，前端报错只能靠这条通道留存证据；
 * 仅在各 WebView 入口初始化时安装一次，组件内不要重复安装。
 */
import { reportFrontendError, type FrontendErrorReport } from '@/bridge/commands'

/** 相同错误的最小上报间隔（毫秒），避免渲染循环刷屏。 */
const DEDUP_WINDOW_MS = 5_000
/** 单条消息 / 堆栈 / 地址的前端截断上限（后端会再次截断）。 */
const MAX_MESSAGE_CHARS = 2_000
const MAX_STACK_CHARS = 8_000
const MAX_URL_CHARS = 512
/** 去重表条目上限，超出后清理过窗口条目。 */
const DEDUP_TABLE_LIMIT = 200

/** 已上报错误的去重表：`scope|message` -> 最近上报时间戳。 */
const lastReported = new Map<string, number>()

/** 按字符数截断，超长时保留前缀并追加省略号（避免破坏代理对）。 */
function truncate(text: string, maxChars: number): string {
  const chars = Array.from(text)
  return chars.length <= maxChars ? text : `${chars.slice(0, maxChars).join('')}…`
}

/** 构造并发送一条上报；同一 `scope+message` 在去重窗口内只报一次。 */
function report(scope: string, message: string, stack?: string, url?: string): void {
  const key = `${scope}|${message}`
  const now = Date.now()
  const last = lastReported.get(key)
  if (last !== undefined && now - last < DEDUP_WINDOW_MS) return
  lastReported.set(key, now)
  if (lastReported.size > DEDUP_TABLE_LIMIT) {
    for (const [entryKey, at] of lastReported) {
      if (now - at >= DEDUP_WINDOW_MS) lastReported.delete(entryKey)
    }
  }

  const payload: FrontendErrorReport = {
    scope,
    message: truncate(message, MAX_MESSAGE_CHARS),
  }
  if (stack) payload.stack = truncate(stack, MAX_STACK_CHARS)
  if (url) payload.url = truncate(url, MAX_URL_CHARS)
  void reportFrontendError(payload).catch((error) => {
    // 上报通道自身失败时只剩控制台（开发期可见），不再递归上报。
    console.error('[frontend-logger] 上报失败:', error)
  })
}

/**
 * 安装全局错误处理器。
 *
 * 参数：`scope` —— 窗口标识（search-window / settings-window），用于区分来源。
 * 返回：显式上报函数，供入口处 `init().catch` 这类已捕获错误使用。
 */
export function installFrontendErrorLogger(
  scope: string,
): (message: string, stack?: string) => void {
  window.addEventListener('error', (event) => {
    const error: unknown = event.error
    const message = error instanceof Error ? error.message : event.message || String(error)
    const stack = error instanceof Error ? error.stack : undefined
    report(scope, `Uncaught: ${message}`, stack, event.filename)
  })

  window.addEventListener('unhandledrejection', (event) => {
    const reason: unknown = event.reason
    const message = reason instanceof Error ? reason.message : String(reason)
    const stack = reason instanceof Error ? reason.stack : undefined
    report(scope, `Unhandled rejection: ${message}`, stack)
  })

  return (message: string, stack?: string) => report(scope, message, stack)
}

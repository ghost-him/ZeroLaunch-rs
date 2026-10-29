/**
 * 将未知错误转为可读错误文案。
 * BridgeError 取 message；对象/异常取 message 字段；普通值取 String；
 * 避免 String({...}) 产出 "[object Object]" 之类的无意义文案。
 */
export function formatErrorMessage(e: unknown): string {
  if (typeof e === 'string') return e
  if (e instanceof Error) return e.message
  if (typeof e === 'object' && e !== null && 'message' in e) {
    const msg = e.message
    if (typeof msg === 'string' && msg) return msg
  }
  return String(e)
}

/** 错误码 → i18n 键后缀：键为 error.<后缀>，与 locales 中的 error 段落一一对应。 */
const ERROR_CODE_I18N_KEY: Record<string, string> = {
  INVALID_QUERY: 'invalidQuery',
  COMPONENT_NOT_FOUND: 'componentNotFound',
  VALIDATION_FAILED: 'validationFailed',
  ACTION_FAILED: 'actionFailed',
  PLUGIN_ERROR: 'pluginError',
  CONFIG_ERROR: 'configError',
  NETWORK_ERROR: 'networkError',
  INTERNAL_ERROR: 'internalError',
}

/**
 * 将错误码转为用户可读标题。
 * 参数：code - BridgeError 的错误码；t - i18n 翻译函数。
 * 返回：已登记错误码对应的译文；未登记的错误码原样返回（如 ALREADY_INSTALLED，
 * 由调用方按业务语义另行处理），使未知码仍可见而非被吞成空串。
 */
export function formatErrorTitle(code: string, t: (key: string) => string): string {
  const suffix = ERROR_CODE_I18N_KEY[code]
  return suffix ? t(`error.${suffix}`) : code
}

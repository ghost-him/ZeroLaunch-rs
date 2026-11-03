import { computed, reactive } from 'vue'
import { useDialog } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import type { UnsavedChangesRegistry, UnsavedFormEntry } from '../utils/unsavedChanges'

/**
 * 设置页未保存修改的统一登记与离开确认。
 *
 * 各表单在挂载时登记自身的脏状态与丢弃回调，页面切换与关闭窗口前调用
 * `confirmDiscard`：无修改直接放行，有修改弹出「丢弃 / 取消」二次确认。
 */
export function useUnsavedChanges(): UnsavedChangesRegistry {
  const dialog = useDialog()
  const { t } = useI18n()
  /** 已挂载表单的登记表（值不深代理，只跟踪增删与各表单的脏状态）。 */
  const entries = reactive(new Map<string, UnsavedFormEntry>())
  /** 正在显示的确认框（连点导航时复用同一个，避免叠出多个弹窗）。 */
  let pending: Promise<boolean> | null = null

  const hasUnsaved = computed(() => {
    for (const entry of entries.values()) {
      if (entry.isDirty()) return true
    }
    return false
  })

  function register(entry: UnsavedFormEntry): void {
    entries.set(entry.componentId, entry)
  }

  function unregister(componentId: string): void {
    entries.delete(componentId)
  }

  /** 弹出「丢弃 / 取消」确认：左侧丢弃（恢复全部表单），右侧取消（留在原处）。 */
  function ask(): Promise<boolean> {
    const { promise, resolve } = Promise.withResolvers<boolean>()
    let settled = false
    /** 只结算一次：点击按钮后对话框关闭还会再触发一次 onClose。 */
    const settle = (proceed: boolean): void => {
      if (settled) return
      settled = true
      if (proceed) {
        for (const entry of entries.values()) entry.discard()
      }
      resolve(proceed)
    }
    dialog.warning({
      title: t('settings.unsavedTitle'),
      content: t('settings.unsavedContent'),
      positiveText: t('common.cancel'),
      negativeText: t('settings.unsavedDiscard'),
      maskClosable: false,
      closeOnEsc: false,
      onPositiveClick: () => settle(false),
      onNegativeClick: () => settle(true),
      onClose: () => settle(false),
    })
    return promise
  }

  function confirmDiscard(): Promise<boolean> {
    if (!hasUnsaved.value) return Promise.resolve(true)
    if (!pending) {
      pending = ask().finally(() => {
        pending = null
      })
    }
    return pending
  }

  return { register, unregister, hasUnsaved, confirmDiscard }
}

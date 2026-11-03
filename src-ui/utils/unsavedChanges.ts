import type { ComputedRef, InjectionKey } from 'vue'

/** 单个表单的未保存状态登记项。 */
export interface UnsavedFormEntry {
  /** 组件 id（同一组件的表单只保留一份登记）。 */
  componentId: string
  /** 是否存在未保存修改（通常是 computed，读取即建立依赖）。 */
  isDirty: () => boolean
  /** 丢弃未保存修改，把表单恢复到已保存的值。 */
  discard: () => void
}

/** 设置页未保存修改的登记与离开确认。 */
export interface UnsavedChangesRegistry {
  /** 登记表单（挂载时调用，同一组件重复登记以最后一次为准）。 */
  register: (entry: UnsavedFormEntry) => void
  /** 注销表单（卸载时调用）。 */
  unregister: (componentId: string) => void
  /** 当前是否存在任何未保存修改。 */
  hasUnsaved: ComputedRef<boolean>
  /**
   * 离开当前页面或隐藏窗口前的确认。
   * 返回：true 表示可以继续（无未保存修改，或用户选择丢弃并已恢复表单）；
   *      false 表示用户取消，调用方应留在原处。
   */
  confirmDiscard: () => Promise<boolean>
}

export const UNSAVED_CHANGES_KEY: InjectionKey<UnsavedChangesRegistry> = Symbol('unsavedChanges')

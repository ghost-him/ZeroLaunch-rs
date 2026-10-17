import type { InjectionKey, Ref } from 'vue'

/** 待揭示的设置位置。 */
export interface FieldRevealTarget {
  componentId: string
  /** 字段 key；空串表示只定位到组件。 */
  fieldKey: string
}

export interface FieldRevealAccessor {
  /** 当前待揭示位置；null 表示无。 */
  target: Ref<FieldRevealTarget | null>
  /**
   * 请求揭示某个设置位置（会顺带切到该组件所在的分类）。
   * 返回是否找到了可显示该组件的分类；false 表示该组件不在任何分类视图里。
   */
  request: (componentId: string, fieldKey: string) => boolean
}

export const FIELD_REVEAL_KEY: InjectionKey<FieldRevealAccessor> = Symbol('fieldReveal')

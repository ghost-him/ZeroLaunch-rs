import type { InjectionKey, Ref } from 'vue'

export interface FormValuesAccessor {
  getValue: (key: string) => unknown
  setValue: (key: string, val: unknown) => void
  /** 响应式原始值引用，供 computed 中直接读取以建立依赖追踪 */
  values: Ref<Record<string, unknown>>
}

export const FORM_VALUES_KEY: InjectionKey<FormValuesAccessor> = Symbol('formValues')

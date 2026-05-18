import type { InjectionKey } from 'vue'

export interface FormValuesAccessor {
  getValue: (key: string) => unknown
  setValue: (key: string, val: unknown) => void
}

export const FORM_VALUES_KEY: InjectionKey<FormValuesAccessor> = Symbol('formValues')

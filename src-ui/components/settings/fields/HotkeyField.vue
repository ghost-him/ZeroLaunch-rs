<template>
  <div class="hotkey-field" @keydown="handleKeydown">
    <n-input
      :value="displayValue"
      :placeholder="recording ? t('settings.hotkeyPressToRecord') : t('settings.hotkeyPlaceholder')"
      readonly
      clearable
      :disabled="field.readOnly"
      :class="{ 'hotkey-recording': recording }"
      @focus="startRecording"
      @blur="stopRecording"
      @clear="handleClear"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { NInput } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import type { FieldConfig } from '../../../utils/schemaTypes'

const props = defineProps<{
  field: FieldConfig
  /** 与其它字段组件保持一致的接口；hotkey 字段本身不需要 config action。 */
  componentId: string
  modelValue: unknown
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()

const { t } = useI18n()

/** 是否处于录制态：聚焦进入，提交/取消/失焦退出。 */
const recording = ref(false)
/** 录制过程中已按下的组合，实时回显给用户。 */
const pendingCombo = ref('')

/** 展示值：录制中显示已按组合，否则显示已保存的快捷键。 */
const displayValue = computed(() =>
  recording.value ? pendingCombo.value : String(props.modelValue ?? ''),
)

/** 进入录制态，开始捕获按键组合。 */
function startRecording(): void {
  if (props.field.readOnly) return
  recording.value = true
  pendingCombo.value = ''
}

/** 退出录制态并清空临时状态（不修改已保存值）。 */
function stopRecording(): void {
  recording.value = false
  pendingCombo.value = ''
}

/** 点击清除按钮：清空快捷键值。 */
function handleClear(): void {
  emit('update:modelValue', '')
}

/** 处理录制中的按键：Escape 取消、Backspace/Delete 清空、组合键提交。 */
function handleKeydown(e: KeyboardEvent): void {
  if (!recording.value) return
  if (e.key === 'Escape') {
    e.preventDefault()
    ;(e.target as HTMLElement).blur()
    return
  }
  if (e.key === 'Backspace' || e.key === 'Delete') {
    e.preventDefault()
    emit('update:modelValue', '')
    ;(e.target as HTMLElement).blur()
    return
  }
  // 长按产生的重复 keydown 不重复处理：组合以首次按下为准（repeat 事件在
  // normalizeKey 之前拦截，纯修饰键长按也只回显一次等待态）。
  if (e.repeat) return
  const key = normalizeKey(e)
  if (!key) {
    // 纯修饰键：回显等待态（如 "Ctrl+…"）提示继续按主键，不提交、不退出录制。
    const modifiers = formatModifiers(e)
    if (modifiers) {
      pendingCombo.value = `${modifiers}+…`
    }
    return
  }
  e.preventDefault()
  pendingCombo.value = formatCombo(e, key)
  emit('update:modelValue', pendingCombo.value)
  ;(e.target as HTMLElement).blur()
}

/** 将按键事件规范化为主键名（基于 code，不受键盘布局与 Shift 影响）；纯修饰键返回 null。 */
function normalizeKey(e: KeyboardEvent): string | null {
  const modKeys = ['Control', 'Alt', 'Shift', 'Meta', 'OS', 'AltGraph', 'ContextMenu']
  if (modKeys.includes(e.key)) return null
  const code = e.code
  const letter = /^Key([A-Z])$/.exec(code)
  if (letter) return letter[1]
  const digit = /^Digit([0-9])$/.exec(code)
  if (digit) return digit[1]
  if (/^F([1-9]|1[0-2])$/.test(code)) return code
  const named = ['Space', 'Tab', 'CapsLock', 'Enter', 'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Home', 'End', 'PageUp', 'PageDown', 'Insert', 'Delete']
  return named.includes(code) ? code : null
}

/** 按后端格式组合修饰键前缀：顺序 Ctrl、Alt、Shift、Meta，加号连接（不含主键）。 */
function formatModifiers(e: KeyboardEvent): string {
  const parts: string[] = []
  if (e.ctrlKey) parts.push('Ctrl')
  if (e.altKey) parts.push('Alt')
  if (e.shiftKey) parts.push('Shift')
  if (e.metaKey) parts.push('Meta')
  return parts.join('+')
}

/** 按后端格式组合快捷键字符串：修饰键顺序 Ctrl、Alt、Shift、Meta，加号连接。 */
function formatCombo(e: KeyboardEvent, key: string): string {
  const parts = formatModifiers(e)
  return parts ? `${parts}+${key}` : key
}
</script>

<style scoped>
.hotkey-field {
  width: 100%;
}

/* 录制态高亮边框提示（组件级覆写 naive-ui 样式） */
.hotkey-recording :deep(.n-input__border) {
  border-color: var(--primary-color);
}
</style>

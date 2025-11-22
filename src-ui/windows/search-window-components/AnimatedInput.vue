<template>
  <div class="hybrid-input-wrapper">
    <span
      ref="measurerRef"
      :style="sharedStyles"
      class="text-measurer"
    >{{ textBefore }}</span>

    <span
      class="animated-caret"
      :class="{
        'blinking': isFocused,
        'is-moving': isCaretMoving,
        'animated': props.dynamic
      }"
      :style="{ left: caretLeft + 'px', backgroundColor: props.color }"
    />

    <input
      ref="realInputRef"
      v-model="modelValue"
      class="real-input" 
      :placeholder="props.placeholder" 
      :style="[
        sharedStyles,
        {'--placeholder-color': props.placeholderColor, 'caret-color': 'transparent'}]" 
      @click="updateCursorPosition" 
      @select="updateCursorPosition"
      @keyup="updateCursorPosition"
      @keydown="updateCursorPosition" 
      @input="updateCursorPosition"
      @focus="isFocused = true"
      @blur="isFocused = false"
    >
  </div>
</template>

<script setup lang="ts">
import { ref, computed, nextTick, watch } from 'vue'

const props = withDefaults(defineProps<{
    placeholder?: string;
    fontSize?: string;
    color?: string;
    fontFamily?: string;
    placeholderColor?: string;
    dynamic?: boolean;
}>(), {
    fontSize: '16px',
    fontFamily: 'inherit',
    color: 'black',
    placeholderColor: '#999',
    dynamic: true,
})

const sharedStyles = computed(() => ({
    fontFamily: props.fontFamily,
    fontSize: props.fontSize,
    color: props.color,
    letterSpacing: 'normal', // Ensure consistent spacing
}))

const modelValue = defineModel<string>({ required: true })
const currentInputValue = ref(modelValue.value)

const realInputRef = ref<HTMLInputElement | null>(null)
const measurerRef = ref<HTMLElement | null>(null)

const cursorPosition = ref(0)
const caretLeft = ref(0)
const isFocused = ref(false)
const isCaretMoving = ref(false)
let caretMoveTimer: ReturnType<typeof setTimeout> | null = null

const textBefore = computed(() => currentInputValue.value.slice(0, cursorPosition.value))

const updateCursorPosition = async () => {
    if (!realInputRef.value || !measurerRef.value) return

    const input = realInputRef.value
    currentInputValue.value = input.value

    let pos = input.selectionStart || 0

    // Handle selection direction
    if (input.selectionDirection === 'backward') {
        pos = input.selectionStart || 0
    } else if (input.selectionDirection === 'forward') {
        pos = input.selectionEnd || 0
    } else {
        // Default behavior if direction is not supported or 'none'
        pos = input.selectionStart || 0
        if (input.selectionStart !== input.selectionEnd) {
            pos = input.selectionEnd || 0
        }
    }

    cursorPosition.value = pos

    await nextTick()

    const newCaretLeft = measurerRef.value.offsetWidth
    if (caretLeft.value !== newCaretLeft) {
        isCaretMoving.value = true
        clearTimeout(caretMoveTimer ?? undefined)

        caretLeft.value = newCaretLeft

        caretMoveTimer = setTimeout(() => {
            isCaretMoving.value = false
        }, 50)
    }
}

defineExpose({
    realInputRef,
    focus: () => realInputRef.value?.focus(),
    cursorPosition,
})

watch(modelValue, async () => {
    await nextTick()
    await updateCursorPosition()
})
</script>

<style scoped>
.hybrid-input-wrapper {
    position: relative;
    width: 100%;
    height: 100%;
    font-weight: 600;
    line-height: normal;
}

.text-measurer {
    position: absolute;
    left: 0;
    top: 0;
    visibility: hidden;
    white-space: pre;
    pointer-events: none;
    height: 100%;
    display: inline-block;
    align-items: center;
}

.real-input {
    width: 100%;
    height: 100%;
    border: none;
    outline: none;
    padding: 0;
    margin: 0;
    background: transparent;
    font: inherit;
    -webkit-app-region: no-drag;
}

.real-input::placeholder {
    color: var(--placeholder-color, #999);
    opacity: 0.8;
}

.animated-caret {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    width: 1.5px;
    height: 75%;
    background-color: currentColor;
    opacity: 0;
    pointer-events: none;
}

.animated-caret.animated {
    transition: left 0.05s cubic-bezier(.25,.1,.25,1);
}

.animated-caret.blinking {
    opacity: 1;
    animation: blink 1s infinite step-end;
}

.animated-caret.is-moving {
    animation: none;
    opacity: 1;
}

@keyframes blink {
    0%, 100% { opacity: 1; }
    50% { opacity: 0; }
}
</style>

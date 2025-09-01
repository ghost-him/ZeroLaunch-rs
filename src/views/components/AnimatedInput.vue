<template>
    <div class="hybrid-input-wrapper">
        <span ref="measurerRef" :style="sharedStyles" class="text-measurer" >{{ textBefore }}</span>

        <span class="animated-caret" :class="{
            'blinking': isFocused,
            'is-moving': isCaretMoving,
            'animated': props.dynamic
        }" :style="{ left: caretLeft + 'px', backgroundColor: props.color, }"></span>

        <input ref="realInputRef" v-model="modelValue"
            @click="updateCursorPosition" @select="updateCursorPosition" @keyup="updateCursorPosition" @keydown="updateCursorPosition" @focus="isFocused = true"
            @blur="isFocused = false" class="real-input" :placeholder="props.placeholder" :style="[
                sharedStyles,
                {'--placeholder-color': props.placeholderColor}]" />
    </div>
</template>

<script setup lang="ts">
import { ref, computed, nextTick, watch } from 'vue';

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
});

const sharedStyles = computed(() => ({
    fontFamily: props.fontFamily,
    fontSize: props.fontSize,
    color: props.color,
}));

const modelValue = defineModel<string>({ required: true });

const realInputRef = ref<HTMLInputElement | null>(null);
const measurerRef = ref<HTMLElement | null>(null);

const cursorPosition = ref(0);
const caretLeft = ref(0);
const isFocused = ref(false);
const isCaretMoving = ref(false);
let caretMoveTimer: ReturnType<typeof setTimeout> | null = null;

const textBefore = computed(() => modelValue.value.slice(0, cursorPosition.value));

const updateCursorPosition = async () => {
    if (!realInputRef.value || !measurerRef.value || !isFocused.value) return;

    cursorPosition.value = realInputRef.value.selectionStart || 0;

    await nextTick();

    const newCaretLeft = measurerRef.value.offsetWidth;
    if (caretLeft.value !== newCaretLeft) {
        isCaretMoving.value = true;
        clearTimeout(caretMoveTimer ?? undefined);

        caretLeft.value = newCaretLeft;

        caretMoveTimer = setTimeout(() => {
            isCaretMoving.value = false;
        }, 50);
    }
};

const focusInput = () => {
    realInputRef.value?.focus();
};

watch(modelValue, async () => {
    await updateCursorPosition();
});

defineExpose({
    focus: focusInput,
    cursorPosition,
    realInputRef,
});
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
    display: inline-flex;
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
    caret-color: transparent;
    font: inherit;
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
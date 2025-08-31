<template>
    <div class="animated-input-wrapper" @click="focusInput" :style="{ fontFamily: fontFamily }">
        <div class="display-area" :style="{ fontSize: fontSize, color: color }">
            <span ref="textBeforeRef" class="text-segment">{{ textBefore }}</span>
            <span class="animated-caret" :class="{
                'blinking': isFocused,
                'is-moving': isCaretMoving,
                'animated': dynamic
            }" :style="{ left: caretLeft + 'px' }"></span>
            <span class="text-segment">{{ textAfter }}</span>
            <span v-if="!modelValue" class="placeholder" :style="{ color: placeholderColor }">{{ placeholder }}</span>
        </div>
        <input ref="realInputRef" v-model="modelValue" @input="handleInput" @keydown="handleInput"
            @click="updateCursorPosition" @select="updateCursorPosition" @focus="isFocused = true"
            @blur="isFocused = false" class="real-input" />
    </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted } from 'vue';

const props = withDefaults(defineProps<{
    placeholder?: string;
    fontSize?: string;
    color?: string;
    fontFamily?: string;
    placeholderColor?: string;
    dynamic?: boolean;
}>(), {
    dynamic: true
});

const modelValue = defineModel<string>({ required: true });

const realInputRef = ref<HTMLInputElement | null>(null);
const textBeforeRef = ref<HTMLElement | null>(null);

const cursorPosition = ref(0);
const caretLeft = ref(0);
const isFocused = ref(false);
const isCaretMoving = ref(false);
let caretMoveTimer: ReturnType<typeof setTimeout> | null = null;

const textBefore = computed(() => modelValue.value.slice(0, cursorPosition.value));
const textAfter = computed(() => modelValue.value.slice(cursorPosition.value));

const updateCursorPosition = async () => {
    if (!realInputRef.value || !textBeforeRef.value) return;
    cursorPosition.value = realInputRef.value.selectionStart || 0;

    await nextTick();

    const newCaretLeft = textBeforeRef.value.offsetWidth;

    if (caretLeft.value !== newCaretLeft) {
        // 无论是否启用动态效果，移动时都应暂时停止闪烁
        isCaretMoving.value = true;
        clearTimeout(caretMoveTimer ?? undefined);

        caretLeft.value = newCaretLeft;

        // 设置一个短暂的计时器，在移动结束后恢复闪烁动画
        caretMoveTimer = setTimeout(() => {
            isCaretMoving.value = false;
        }, 50);
    }
};

const handleInput = () => {
    setTimeout(updateCursorPosition, 0);
};

watch(modelValue, () => {
    handleInput();
});

onMounted(() => {
    updateCursorPosition();
});

const focusInput = () => {
    realInputRef.value?.focus();
};

defineExpose({
    focus: focusInput,
    cursorPosition
});
</script>

<style scoped>
.animated-input-wrapper {
    position: relative;
    width: 100%;
    height: 100%;
    cursor: text;
    background: transparent;
}

.display-area {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    white-space: nowrap;
    overflow: hidden;
    font-weight: 600;
    line-height: normal;
    pointer-events: none;
}

.text-segment {
    white-space: pre;
}

.placeholder {
    position: absolute;
    left: 0;
    opacity: 0.8;
    pointer-events: none;
}

.animated-caret {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    width: 1.5px;
    height: 75%;
    background-color: currentColor;
    opacity: 0;
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

.real-input {
    width: 100%;
    height: 100%;
    border: none;
    outline: none;
    padding: 0;
    margin: 0;
    background: transparent;
    color: transparent;
    caret-color: transparent;
    font: inherit;
    font-weight: 600;
    line-height: normal;
}

@keyframes blink {

    0%,
    100% {
        opacity: 1;
    }

    50% {
        opacity: 0;
    }
}
</style>
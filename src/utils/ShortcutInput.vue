<template>
    <div class="shortcut-input">
        <label class="shortcut-label">{{ label || t('shortcut_input.shortcut') }}</label>
        <div class="key-display" :class="{ 'listening': isListening, 'disabled': disabled }" @click="startListening"
            tabindex="disabled ? -1 : 0">
            <i class="el-icon-keyboard" v-if="!displayValue"></i>
            {{ displayValue || (disabled ? t('shortcut_input.disabled') : t('shortcut_input.click_to_set')) }}
        </div>
    </div>
</template>

<script lang="ts" setup>
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { Shortcut } from '../api/remote_config_types'
import { PropType } from 'vue';

const { t } = useI18n();

const props = defineProps({
    label: {
        type: String,
        default: ''
    },
    modelValue: {
        type: Object as PropType<Shortcut | null>,
        default: () => null
    },
    disabled: {
        type: Boolean,
        default: false
    }
});

const emit = defineEmits(['update:modelValue']);

const shortcut = ref(props.modelValue);
const isListening = ref(false);

watch(() => props.modelValue, (newVal) => {
    shortcut.value = newVal;
});

watch(() => props.disabled, (newVal) => {
    if (newVal && isListening.value) {
        stopListening();
    }
});

// 显示的快捷键文本
const displayValue = computed(() => {
    if (!shortcut.value) return '';

    const keys = [];
    if (shortcut.value.ctrl) keys.push('Ctrl');
    if (shortcut.value.alt) keys.push('Alt');
    if (shortcut.value.shift) keys.push('Shift');
    if (shortcut.value.meta) keys.push('Meta');
    if (shortcut.value.key && !['Control', 'Alt', 'Shift', 'Meta'].includes(shortcut.value.key)) {
        keys.push(shortcut.value.key === ' ' ? 'Space' : shortcut.value.key);
    }

    return keys.join(' + ');
});

// 全局按键处理
const handleDocumentKeyDown = (e: KeyboardEvent) => {
    if (!isListening.value || props.disabled) return;

    e.preventDefault();
    e.stopPropagation();

    const newShortcut = {
        key: e.key === ' ' ? 'Space' : e.key,
        ctrl: e.ctrlKey,
        alt: e.altKey,
        shift: e.shiftKey,
        meta: e.metaKey
    };

    shortcut.value = newShortcut;
    emit('update:modelValue', newShortcut);
}

const handleDocumentKeyUp = () => {
    if (isListening.value) {
        stopListening();
    }
}

// 开始监听
function startListening() {
    if (isListening.value || props.disabled) return;

    isListening.value = true;
    document.addEventListener('keydown', handleDocumentKeyDown);
    document.addEventListener('keyup', handleDocumentKeyUp);
}

// 停止监听
function stopListening() {
    if (isListening.value) {
        document.removeEventListener('keydown', handleDocumentKeyDown);
        document.removeEventListener('keyup', handleDocumentKeyUp);
        isListening.value = false;
    }
}

// 点击外部区域停止监听
function handleClickOutside(event: MouseEvent) {
    if (isListening.value && !(event.target as Element)?.closest('.shortcut-input')) {
        stopListening();
    }
}

onMounted(() => {
    document.addEventListener('click', handleClickOutside);
});

onUnmounted(() => {
    document.removeEventListener('click', handleClickOutside);
    stopListening(); // 确保卸载时清理监听器
});

defineExpose({
    stopListening
})
</script>

<style scoped>
.key-display.disabled {
    cursor: not-allowed;
    opacity: 0.6;
}

.shortcut-input {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-bottom: 16px;
    font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
}

.shortcut-label {
    font-size: 14px;
    font-weight: 500;
    color: #2c3e50;
    min-width: 80px;
}

.key-display {
    padding: 8px 14px;
    border: 1px solid #e0e3e9;
    border-radius: 6px;
    min-width: 180px;
    text-align: center;
    cursor: pointer;
    user-select: none;
    background-color: #f9fafc;
    transition: all 0.25s ease;
    font-size: 14px;
    color: #606266;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
}

.key-display:hover {
    background-color: #f0f2f5;
    border-color: #c0c4cc;
}

.key-display:focus {
    outline: none;
    border-color: #409eff;
    box-shadow: 0 0 0 2px rgba(64, 158, 255, 0.2);
}

.key-display.listening {
    border-color: #409eff;
    background-color: #ecf5ff;
    box-shadow: 0 0 0 3px rgba(64, 158, 255, 0.25);
    color: #409eff;
    font-weight: 500;
}

.el-icon-keyboard {
    font-size: 16px;
    opacity: 0.7;
}
</style>
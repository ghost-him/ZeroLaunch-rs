<template>
    <div class="shortcut-input">
        <label>{{ label }}</label>
        <div class="key-display" :class="{ 'listening': isListening }" @click="startListening" tabindex="0">
            {{ displayValue || '点击设置快捷键' }}
        </div>
        <el-button v-if="shortcut" @click="resetShortcut" class="reset-btn">重置</el-button>
    </div>
</template>

<script lang="ts" setup>
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { ElButton } from 'element-plus';
import { Shortcut } from '../api/remote_config_types'
import { PropType } from 'vue';


const props = defineProps({
    label: {
        type: String,
        default: '快捷键'
    },
    modelValue: {
        type: Object as PropType<Shortcut | null>,
        default: () => null
    },
    defaultValue: {
        type: Object as PropType<Shortcut | null>,
        default: () => null
    }
});

const emit = defineEmits(['update:modelValue', 'before-change', 'after-change']);

const shortcut = ref(props.modelValue);
const isListening = ref(false);

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
    if (!isListening.value) return;

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

const handleDocumentKeyUp = (e: KeyboardEvent) => {
    if (isListening.value) {
        stopListening();
    }
}

// 开始监听
function startListening() {
    if (isListening.value) return;

    emit('before-change');
    isListening.value = true;
    document.addEventListener('keydown', handleDocumentKeyDown);
    document.addEventListener('keyup', handleDocumentKeyUp);
}

// 停止监听
function stopListening() {
    isListening.value = false;
    document.removeEventListener('keydown', handleDocumentKeyDown);
    document.removeEventListener('keyup', handleDocumentKeyUp);
    emit('after-change');
}

// 重置快捷键
function resetShortcut() {
    shortcut.value = props.defaultValue;
    emit('update:modelValue', props.defaultValue);
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
</script>

<style scoped>
/* 原有样式保持不变 */
.shortcut-input {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 10px;
}

.key-display {
    padding: 5px 10px;
    border: 1px solid #dcdfe6;
    border-radius: 4px;
    min-width: 150px;
    text-align: center;
    cursor: pointer;
    user-select: none;
    background-color: #ffffff;
    transition: border-color 0.2s, box-shadow 0.2s;
}

.key-display.listening {
    border-color: #409eff;
    box-shadow: 0 0 0 2px rgba(64, 158, 255, 0.2);
}

.reset-btn {
    padding: 2px 8px;
    font-size: 12px;
    background-color: transparent;
    border: none;
    cursor: pointer;
    color: #606266;
}

.reset-btn:hover {
    color: #409eff;
}
</style>
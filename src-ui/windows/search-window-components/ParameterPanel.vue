<template>
  <div class="parameter-panel" :style="panelStyle">
    <div class="parameter-panel__header">
      <div class="parameter-panel__title">{{ prompt }}</div>
      <div class="parameter-panel__progress">
        {{ progress }}
      </div>
    </div>
    <input
      ref="inputRef"
      v-model="inputValue"
      class="parameter-panel__input"
      type="text"
      :placeholder="t('parameter.input_placeholder')"
      @keydown.enter="onConfirm"
      @keydown.esc="onCancel"
    />
    <div class="parameter-panel__tips">{{ t('parameter.hint') }}</div>
    <div class="parameter-panel__actions">
      <button type="button" class="parameter-panel__button secondary" @click="onCancel">
        {{ t('parameter.cancel') }}
      </button>
      <button type="button" class="parameter-panel__button primary" @click="onConfirm">
        {{ actionLabel }}
      </button>
    </div>
    <div class="parameter-panel__preview">
      <div class="parameter-panel__preview-label">{{ t('parameter.preview') }}</div>
      <pre class="parameter-panel__preview-content">{{ preview }}</pre>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { UIConfig } from '../../api/remote_config_types';

const { t } = useI18n();

const props = defineProps<{
  uiConfig: UIConfig;
  prompt: string;
  progress: string;
  actionLabel: string;
  preview: string;
}>();

const inputValue = defineModel<string>('inputValue', { required: true });
const emit = defineEmits<{
  (e: 'confirm'): void;
  (e: 'cancel'): void;
}>();

const inputRef = ref<HTMLInputElement | null>(null);

const onConfirm = () => {
  emit('confirm');
};

const onCancel = () => {
  emit('cancel');
};

const focus = () => {
  inputRef.value?.focus();
};

const panelStyle = computed(() => ({
  backgroundColor: props.uiConfig.search_bar_background_color,
  borderColor: props.uiConfig.selected_item_color,
  color: props.uiConfig.item_font_color,
}));

defineExpose({
  focus
});
</script>

<style scoped>
.parameter-panel {
  margin: 12px;
  padding: 16px;
  border-radius: 12px;
  border: 1px solid transparent;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.parameter-panel__header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  font-size: 14px;
}

.parameter-panel__title {
  font-weight: 600;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.parameter-panel__progress {
  font-variant-numeric: tabular-nums;
  font-size: 13px;
}

.parameter-panel__input {
  width: 100%;
  padding: 10px 12px;
  border-radius: 8px;
  border: 1px solid rgba(0, 0, 0, 0.1);
  font-size: 14px;
  outline: none;
  box-sizing: border-box;
}

.parameter-panel__input:focus {
  border-color: rgba(99, 102, 241, 0.6);
  box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.15);
}

.parameter-panel__tips {
  font-size: 12px;
  opacity: 0.7;
}

.parameter-panel__actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.parameter-panel__button {
  border: none;
  border-radius: 8px;
  padding: 8px 16px;
  font-size: 13px;
  cursor: pointer;
  transition: transform 0.15s ease, box-shadow 0.15s ease;
}

.parameter-panel__button.primary {
  background-color: rgba(99, 102, 241, 0.9);
  color: #fff;
}

.parameter-panel__button.secondary {
  background-color: rgba(0, 0, 0, 0.05);
  color: inherit;
}

.parameter-panel__button:active {
  transform: scale(0.98);
}

.parameter-panel__preview {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.parameter-panel__preview-label {
  font-size: 12px;
  opacity: 0.7;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.parameter-panel__preview-content {
  margin: 0;
  padding: 12px;
  border-radius: 8px;
  background-color: rgba(0, 0, 0, 0.05);
  font-family: 'Cascadia Code', 'Consolas', monospace;
  font-size: 13px;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>

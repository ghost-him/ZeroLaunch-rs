<template>
  <div class="search-bar-wrapper">
    <n-input
      ref="inputRef"
      v-model:value="inputText"
      type="text"
      placeholder="Hello, ZeroLaunch! ヾ(≧▽≦*)o"
      :autofocus="true"
      size="large"
      @update:value="onInput"
    >
      <!-- 无边框无图标的纯净设计 -->
    </n-input>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, nextTick } from 'vue'
import { NInput } from 'naive-ui'
import { useSearch } from '../../composables/useSearch'

const { handleInput } = useSearch()

const inputText = ref('')
const inputRef = ref<InstanceType<typeof NInput> | null>(null)

function onInput(value: string) {
  inputText.value = value
  handleInput(value)
}

onMounted(async () => {
  await nextTick()
  inputRef.value?.focus()
})
</script>

<style scoped>
.search-bar-wrapper {
  height: var(--search-bar-height);
  flex-shrink: 0;
  padding: 16px 24px; /* Larger horizontal padding */
  display: flex;
  align-items: center;
  position: relative;
  z-index: 10;
  /* Soft hierarchical shadow indicating separation without hard lines */
  box-shadow: var(--shadow-header);
}

.search-bar-wrapper :deep(.n-input) {
  --n-border: transparent !important;
  --n-border-hover: transparent !important;
  --n-border-focus: transparent !important;
  --n-box-shadow-focus: transparent !important;
  --n-color: transparent !important;
  --n-color-focus: transparent !important;
  --n-text-color: var(--text-primary);
  --n-placeholder-color: var(--text-secondary);
  --n-font-size: var(--font-size-xl) !important;
  --n-height: 40px !important;
  --n-caret-color: var(--accent-color) !important;
  --n-padding-left: 0 !important; /* Flush with wrapper padding */
}
</style>

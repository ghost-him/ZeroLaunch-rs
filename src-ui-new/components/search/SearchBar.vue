<template>
  <div class="search-bar-wrapper">
    <n-input
      ref="inputRef"
      v-model:value="inputText"
      type="text"
      placeholder="输入关键字搜索..."
      :autofocus="true"
      size="large"
      clearable
      @update:value="onInput"
    >
      <template #prefix>
        <n-icon :size="18" color="var(--text-secondary)">
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="11" cy="11" r="8" />
            <path d="m21 21-4.3-4.3" />
          </svg>
        </n-icon>
      </template>
    </n-input>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, nextTick } from 'vue'
import { NInput, NIcon } from 'naive-ui'
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
  padding: 12px 16px 8px;
}

.search-bar-wrapper :deep(.n-input) {
  --n-border: var(--border-color);
  --n-border-focus: var(--accent-color);
  --n-box-shadow-focus: 0 0 0 2px rgba(32, 128, 240, 0.2);
  --n-border-radius: var(--radius-md);
  --n-color: var(--bg-primary);
  --n-color-focus: var(--bg-primary);
  --n-text-color: var(--text-primary);
  --n-placeholder-color: var(--text-secondary);
}
</style>

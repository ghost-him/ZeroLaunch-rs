<template>
  <img v-if="isImage" :src="src" :width="size" :height="size" alt="" />
  <n-icon v-else-if="src" :size="size" color="var(--text-secondary)">
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <path d="M4 17v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2" />
      <polyline points="7 9 12 4 17 9" />
      <line x1="12" x2="12" y1="4" y2="16" />
    </svg>
  </n-icon>
  <div v-else class="icon-placeholder" :style="{ width: size + 'px', height: size + 'px' }" />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NIcon } from 'naive-ui'

const props = defineProps<{
  src: string
  size?: number
}>()

const isImage = computed(() => props.src && (props.src.startsWith('http') || props.src.startsWith('data:') || props.src.startsWith('/')))
</script>

<style scoped>
.icon-placeholder {
  background: var(--bg-secondary);
  border-radius: 4px;
}
</style>

<template>
  <img v-if="isImage" :src="src" :width="size" :height="size" alt="" />
  <n-icon v-else-if="src" :size="size" color="var(--text-secondary)">
    <Upload />
  </n-icon>
  <div v-else class="icon-placeholder" :style="{ width: size + 'px', height: size + 'px' }" />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NIcon } from 'naive-ui'
import { Upload } from 'lucide-vue-next'

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

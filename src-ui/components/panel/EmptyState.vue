<template>
  <div class="empty-state">
    <n-text depth="2">输入关键字开始搜索</n-text>
  </div>
</template>

<script setup lang="ts">
import { NText } from 'naive-ui'
</script>

<style scoped>
.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 32px;
  color: var(--text-secondary);
  font-size: var(--font-size-base);
}
</style>

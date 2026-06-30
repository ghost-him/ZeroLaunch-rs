<template>
  <div class="loading-indicator">
    <n-spin :size="16" />
  </div>
</template>

<script setup lang="ts">
import { NSpin } from 'naive-ui'
</script>

<style scoped>
.loading-indicator {
  display: flex;
  justify-content: center;
  padding: 16px;
}
</style>

<template>
  <div class="template-panel">
    <!-- 你的自定义面板内容 -->
    <p>Hello from template panel</p>
  </div>
</template>

<script setup lang="ts">
// 自定义面板 Props 契约:
// - data: unknown — 后端 CustomPanel.data
// - actions: ResultAction[] — 后端 CustomPanel.actions
defineProps<{
  data: unknown
  actions: import('@/bridge/contract').ResultAction[]
}>()
</script>

<style scoped>
.template-panel {
  padding: 12px;
}
</style>

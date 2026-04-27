<template>
  <div class="plugin-settings-host">
    <!-- TODO: 后续从 PluginManager 获取注册的自定义设置组件 -->
    <n-text depth="3">此组件使用默认表单</n-text>
  </div>
</template>

<script setup lang="ts">
import { NText } from 'naive-ui'
</script>

<style scoped>
.plugin-settings-host {
  padding: 16px;
}
</style>

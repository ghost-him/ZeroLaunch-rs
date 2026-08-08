<template>
  <section class="debug-card">
    <header class="debug-card__header">
      <h3 class="debug-card__title">{{ title }}</h3>
      <div class="debug-card__actions">
        <slot name="actions" />
      </div>
    </header>
    <p v-if="description" class="debug-card__desc">{{ description }}</p>
    <div class="debug-card__body">
      <slot />
    </div>
  </section>
</template>

<script setup lang="ts">
/** 调试页统一卡片容器：标题 + 可选描述 + 内容插槽，与设置页卡片视觉一致。 */
defineProps<{
  title: string
  description?: string
}>()
</script>

<style scoped>
.debug-card {
  display: flex;
  flex-direction: column;
  background: var(--bg-color-secondary);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  padding: 16px;
  min-height: 0;
  overflow: hidden;
}

.debug-card__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  flex-shrink: 0;
}

.debug-card__title {
  margin: 0;
  font-size: var(--font-size-base);
  font-weight: 600;
  color: var(--text-color);
}

.debug-card__desc {
  margin: 4px 0 0;
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  flex-shrink: 0;
}

.debug-card__body {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding-top: 12px;
}
</style>

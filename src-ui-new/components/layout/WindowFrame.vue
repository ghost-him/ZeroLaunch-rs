<template>
  <div class="window-frame">
    <slot />
  </div>
</template>

<style scoped>
.window-frame {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  overflow: hidden;
}
</style>

<template>
  <div class="window-frame">
    <slot />
  </div>
</template>

<style scoped>
.window-frame {
  display: flex;
  flex-direction: column;
  background: var(--bg-primary);
  border: var(--window-border-width) solid transparent; /* Remove hard border */
  border-radius: 12px;
  overflow: hidden;
  box-shadow: var(--shadow-md); /* Soft hierarchical shadow to replace physical border */
  /* 允许按照内容自行决定高度，但可以限定一下总体下限防止全空 */
  min-height: calc(var(--search-bar-height) + var(--window-border-width) * 2);
}
</style>

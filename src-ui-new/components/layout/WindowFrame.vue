<template>
  <div class="window-frame">
    <slot />
  </div>
</template>

<style scoped>
.window-frame {
  position: relative;
  display: flex;
  flex-direction: column;
  background-color: var(--bg-primary);
  border: var(--window-border-width) solid transparent;
  border-color: var(--window-border-color);
  border-radius: var(--window-corner-radius);
  overflow: hidden;
  min-height: calc(var(--search-bar-height) + var(--window-border-width) * 2);
}

/* 背景图片层：使用伪元素实现透明度，不影响子元素 */
.window-frame::before {
  content: '';
  position: absolute;
  inset: 0;
  background-image: var(--bg-image-url);
  background-size: var(--bg-size);
  background-position: var(--bg-position);
  background-repeat: var(--bg-repeat);
  opacity: var(--bg-image-opacity);
  border-radius: inherit;
  pointer-events: none;
  z-index: 0;
}

.window-frame > :deep(*) {
  position: relative;
  z-index: 1;
}
</style>

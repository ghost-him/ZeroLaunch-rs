<template>
  <div
    v-if="uiConfig.footer_height > 0"
    class="footer"
    :style="{ backgroundColor: uiConfig.search_bar_background_color, fontSize: Math.round(uiConfig.footer_height * uiConfig.footer_font_size * layoutConstants.fontSizeRatio) + 'px', fontFamily: uiConfig.footer_font_family, }"
    @mousedown="startDrag"
  >
    <div class="footer-left">
      <span
        class="status-text"
        :style="{ color: uiConfig.footer_font_color, fontFamily: uiConfig.footer_font_family }"
      >{{
        leftText || appConfig.tips }}</span>
    </div>
    <div class="footer-center" />
    <div class="footer-right">
      <span
        class="open-text"
        :style="{ color: uiConfig.footer_font_color, fontFamily: uiConfig.footer_font_family }"
      >
        {{ statusText }}
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { UIConfig, AppConfig } from '../../api/remote_config_types'
import { getCurrentWindow } from '@tauri-apps/api/window'

const layoutConstants = {
  fontSizeRatio: 0.01,
}

const props = defineProps<{
  uiConfig: UIConfig;
  appConfig: AppConfig;
  statusText: string;
  leftText?: string;
}>()

const startDrag = (e: MouseEvent) => {
  if (!props.appConfig.is_enable_drag_window) return
  if (e.button !== 0) return
  getCurrentWindow().startDragging()
}
</script>

<style scoped>
.footer {
  box-sizing: border-box;
  flex: 1;
  display: flex;
  align-items: center;
  border-top: 1px solid rgba(0, 0, 0, 0.05);
  width: 100%;
}

.footer-left {
  margin-left: 16px;
  flex-shrink: 0;
}

.footer-right {
  margin-right: 16px;
  flex-shrink: 0;
}

.footer-center {
  flex-grow: 1;
}

.status-text,
.open-text {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>

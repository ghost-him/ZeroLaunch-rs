<template>
  <div v-if="uiConfig.footer_height > 0" class="footer drag_area"
    :style="{ backgroundColor: uiConfig.search_bar_background_color, fontSize: Math.round(uiConfig.footer_height * uiConfig.footer_font_size * layoutConstants.fontSizeRatio) + 'px', fontFamily: uiConfig.footer_font_family, }">
    <div class="footer-left">
      <span class="status-text" :style="{ color: uiConfig.footer_font_color }">{{
        leftText || appConfig.tips }}</span>
    </div>
    <div class="footer-center drag_area"></div>
    <div class="footer-right">
      <span class="open-text" :style="{ color: uiConfig.footer_font_color }">
        {{ statusText }}
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { UIConfig, AppConfig } from '../../api/remote_config_types';

const layoutConstants = {
  fontSizeRatio: 0.01
};

defineProps<{
  uiConfig: UIConfig;
  appConfig: AppConfig;
  statusText: string;
  leftText?: string;
}>();
</script>

<style scoped>
.footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 10px;
  width: 100%;
  box-sizing: border-box;
  height: 100%; /* Controlled by parent or style binding */
}

.footer-left,
.footer-right {
  display: flex;
  align-items: center;
  white-space: nowrap;
}

.footer-center {
  flex: 1;
  height: 100%;
}

.drag_area {
  -webkit-app-region: drag;
}
</style>

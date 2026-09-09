<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { NDescriptions, NDescriptionsItem, NTag, NText } from 'naive-ui'
import type { PluginManifest } from '@/bridge/commands'

/** 待安装插件包的 manifest 信息展示 —— 安装确认弹窗复用块。
 *
 * 数据来自后端预检（plugin_inspect_package / market_preview_package），
 * 展示包内 manifest.toml 的真实字段，供用户判断是否安装；
 * 不承载安装动作，仅呈现。manifest 的 name/description 为插件原始值，
 * 尚未安装故无插件语言包翻译，直接展示原文。
 */
const props = defineProps<{ manifest: PluginManifest }>()

const { t } = useI18n()

/** 能力（components.provides）原始标记值，如 Plugin / DataSource。 */
function providesList(): string[] {
  return props.manifest.components.provides ?? []
}
</script>

<template>
  <div class="plugin-package-info">
    <!-- 名称 + 版本头部 -->
    <div class="pkg-head">
      <NText strong>{{ manifest.plugin.name }}</NText>
      <NTag size="small" :bordered="false" type="primary">
        v{{ manifest.plugin.version }}
      </NTag>
    </div>
    <NDescriptions :column="1" bordered size="small" label-placement="left">
      <NDescriptionsItem :label="t('settings.thirdPartyPlugins.fieldPluginId')">
        {{ manifest.plugin.id }}
      </NDescriptionsItem>
      <NDescriptionsItem v-if="manifest.plugin.description" :label="t('settings.thirdPartyPlugins.fieldDescription')">
        {{ manifest.plugin.description }}
      </NDescriptionsItem>
      <NDescriptionsItem :label="t('settings.thirdPartyPlugins.colAuthor')">
        {{ manifest.plugin.author || t('common.notAvailable') }}
      </NDescriptionsItem>
      <NDescriptionsItem v-if="manifest.plugin.license" :label="t('settings.thirdPartyPlugins.fieldLicense')">
        {{ manifest.plugin.license }}
      </NDescriptionsItem>
      <NDescriptionsItem v-if="manifest.plugin.homepage" :label="t('settings.thirdPartyPlugins.fieldHomepage')">
        {{ manifest.plugin.homepage }}
      </NDescriptionsItem>
      <NDescriptionsItem :label="t('settings.thirdPartyPlugins.fieldMinHostVersion')">
        {{ manifest.plugin.minHostVersion }}
      </NDescriptionsItem>
      <NDescriptionsItem :label="t('settings.thirdPartyPlugins.fieldProvides')">
        <NTag v-for="p in providesList()" :key="p" size="small" type="info" style="margin: 2px 6px 2px 0;">
          {{ p }}
        </NTag>
        <NText v-if="providesList().length === 0" depth="3">
          {{ t('common.notAvailable') }}
        </NText>
      </NDescriptionsItem>
    </NDescriptions>
  </div>
</template>

<style scoped>
.pkg-head {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}

/* 长 ID / 主页地址在描述格内换行，避免撑破弹窗 */
.plugin-package-info :deep(.n-descriptions-item-content) {
  word-break: break-all;
}
</style>

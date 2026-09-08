<script setup lang="ts">
import { NModal, NText, NTag, NDescriptions, NDescriptionsItem } from 'naive-ui'
import { useI18n } from 'vue-i18n'

/**
 * 插件介绍弹窗：介绍宿主支持的插件来源与两种形态（行内/独立）及区别。
 * 由插件管理页与插件市场页共用，仅负责展示，形态数据由后端 plugin 详情下发。
 */
const props = defineProps<{
  show: boolean
}>()

const emit = defineEmits<{
  (e: 'update:show', value: boolean): void
}>()

const { t } = useI18n()

/** NModal 显隐由调用方 v-model:show 驱动。 */
function onUpdateShow(value: boolean) {
  emit('update:show', value)
}

/** 对比行数据：维度 + 行内/独立插件各自描述。 */
const diffRows = [
  { label: 'diffWake', inline: 'diffWakeInline', panel: 'diffWakePanel' },
  { label: 'diffUi', inline: 'diffUiInline', panel: 'diffUiPanel' },
  { label: 'diffUse', inline: 'diffUseInline', panel: 'diffUsePanel' },
]
</script>

<template>
  <NModal
    :show="props.show"
    preset="card"
    :title="t('settings.thirdPartyPlugins.intro')"
    style="width: 680px; max-width: calc(100vw - 48px);"
    @update:show="onUpdateShow"
  >
    <div class="plugin-intro-content">
      <section class="intro-section">
        <NText strong>{{ t('settings.thirdPartyPlugins.introWhatTitle') }}</NText>
        <p class="intro-para">{{ t('settings.thirdPartyPlugins.introWhatBody') }}</p>
      </section>

      <section class="intro-section">
        <NText strong>{{ t('settings.thirdPartyPlugins.introFormTitle') }}</NText>
        <div class="form-block">
          <NTag size="small" type="success">{{ t('settings.thirdPartyPlugins.inlineName') }}</NTag>
          <p class="intro-para">{{ t('settings.thirdPartyPlugins.introInlineBody') }}</p>
        </div>
        <div class="form-block">
          <NTag size="small" type="primary">{{ t('settings.thirdPartyPlugins.panelName') }}</NTag>
          <p class="intro-para">{{ t('settings.thirdPartyPlugins.introPanelBody') }}</p>
        </div>
      </section>

      <section class="intro-section">
        <NText strong>{{ t('settings.thirdPartyPlugins.introDiffTitle') }}</NText>
        <NDescriptions
          :column="1"
          bordered
          size="small"
          label-placement="left"
          class="intro-diff"
        >
          <NDescriptionsItem
            v-for="row in diffRows"
            :key="row.label"
            :label="t(`settings.thirdPartyPlugins.${row.label}`)"
          >
            <div class="diff-line">
              <span class="diff-form-name">{{ t('settings.thirdPartyPlugins.inlineName') }}：</span>
              {{ t(`settings.thirdPartyPlugins.${row.inline}`) }}
            </div>
            <div class="diff-line">
              <span class="diff-form-name">{{ t('settings.thirdPartyPlugins.panelName') }}：</span>
              {{ t(`settings.thirdPartyPlugins.${row.panel}`) }}
            </div>
          </NDescriptionsItem>
        </NDescriptions>
      </section>

      <NText depth="3" class="intro-tip">
        {{ t('settings.thirdPartyPlugins.introSeeDetail') }}
      </NText>
    </div>
  </NModal>
</template>

<style scoped>
.plugin-intro-content {
  box-sizing: border-box;
  max-height: min(560px, calc(100vh - 160px));
  overflow-y: auto;
}

.intro-section + .intro-section {
  margin-top: 16px;
}

.intro-section :deep(.n-text--strong) {
  display: block;
  margin-bottom: 8px;
}

.form-block + .form-block {
  margin-top: 12px;
}

.intro-para {
  margin: 8px 0 0;
  line-height: 1.7;
}

.intro-diff {
  margin-top: 8px;
}

.diff-line + .diff-line {
  margin-top: 4px;
}

.diff-form-name {
  font-weight: 600;
}

.intro-tip {
  display: block;
  margin-top: 16px;
}
</style>

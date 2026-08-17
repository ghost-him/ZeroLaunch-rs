<template>
  <div class="about-view">
    <h3 class="about-name">ZeroLaunch-rs</h3>
    <n-text depth="3" class="about-version">v{{ version }}</n-text>

    <p class="about-tagline">{{ t('settings.aboutPage.tagline') }}</p>

    <div class="about-info">
      <div v-for="item in infoRows" :key="item.label" class="info-row">
        <span class="info-label">{{ item.label }}</span>
        <span class="info-value">{{ item.value }}</span>
      </div>
    </div>

    <div class="about-links">
      <a
        v-for="link in links"
        :key="link.label"
        class="about-link"
        href="#"
        @click.prevent="openLink(link.url)"
      >{{ link.label }}</a>
    </div>

    <n-text depth="3" class="about-copyright">{{ t('settings.aboutPage.copyright') }}</n-text>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { NText, useMessage } from 'naive-ui'
import { openExternal } from '@/bridge/commands'

defineProps<{ version: string }>()

const { t } = useI18n()
const message = useMessage()

/** 关于页信息行：技术栈 / 开源协议 / 作者。 */
const infoRows = computed(() => [
  { label: t('settings.aboutPage.stackLabel'), value: t('settings.aboutPage.stackValue') },
  { label: t('settings.aboutPage.licenseLabel'), value: t('settings.aboutPage.licenseValue') },
  { label: t('settings.aboutPage.authorLabel'), value: t('settings.aboutPage.authorValue') },
])

/** 关于页资源链接：点击后用系统浏览器打开。 */
const links = computed(() => [
  { label: t('settings.aboutPage.website'), url: 'https://zerolaunch.ghost-him.com' },
  { label: t('settings.aboutPage.github'), url: 'https://github.com/ghost-him/ZeroLaunch-rs' },
  { label: t('settings.aboutPage.gitee'), url: 'https://gitee.com/ghost-him/ZeroLaunch-rs' },
  { label: t('settings.aboutPage.gitcode'), url: 'https://gitcode.com/ghost-him/ZeroLaunch-rs' },
  { label: t('settings.aboutPage.wiki'), url: 'https://github.com/ghost-him/ZeroLaunch-rs/wiki' },
  { label: t('settings.aboutPage.issues'), url: 'https://github.com/ghost-him/ZeroLaunch-rs/issues' },
])

/** 用系统浏览器打开外链，失败时提示。 */
function openLink(url: string) {
  void openExternal(url).catch(() => {
    message.error(t('settings.aboutPage.openFailed'))
  })
}
</script>

<style scoped>
/* 弹性填充链：settings-content(flex column) → about-view(flex:1 + 内层滚动) */
.about-view {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 32px;
  overflow-y: auto;
}

.about-name {
  margin: 0;
  font-size: var(--font-size-xl);
  font-weight: 600;
  color: var(--text-color);
  flex-shrink: 0;
}

.about-version {
  font-size: var(--font-size-sm);
  flex-shrink: 0;
}

.about-tagline {
  margin: 0;
  color: var(--text-color-secondary);
  font-size: var(--font-size-md);
  text-align: center;
  flex-shrink: 0;
}

.about-info {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 8px;
  flex-shrink: 0;
}

.info-row {
  display: flex;
  gap: 12px;
}

.info-label {
  width: 72px;
  text-align: right;
  color: var(--text-color-secondary);
  font-size: var(--font-size-md);
}

.info-value {
  color: var(--text-color);
  font-size: var(--font-size-md);
}

.about-links {
  display: flex;
  gap: 16px;
  margin-top: 12px;
  flex-shrink: 0;
}

.about-link {
  color: var(--primary-color);
  font-size: var(--font-size-md);
  text-decoration: none;
  cursor: pointer;
}

.about-link:hover {
  text-decoration: underline;
}

.about-copyright {
  margin-top: 12px;
  font-size: var(--font-size-sm);
  flex-shrink: 0;
}
</style>

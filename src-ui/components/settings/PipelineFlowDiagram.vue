<template>
  <div class="pipeline-flow-diagram">
    <!-- 可点击标题栏 -->
    <div class="flow-header" @click="expanded = !expanded">
      <span class="flow-icon" :class="{ rotated: expanded }">▶</span>
      <span class="flow-title">{{ t('settings.pipelineFlow.title') }}</span>
    </div>

    <!-- 可折叠内容区域 -->
    <Transition name="collapse">
      <div v-if="expanded" class="flow-steps">
        <template v-for="(step, i) in steps" :key="step.labelKey">
          <div class="flow-step">
            <div class="step-num">{{ i + 1 }}</div>
            <div class="step-label">{{ t(step.labelKey) }}</div>
            <div class="step-desc">{{ t(step.descKey) }}</div>
          </div>
          <div v-if="i < steps.length - 1" class="flow-arrow">➔</div>
        </template>
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

/// 管道阶段一览（顺序即候选与查询的真实流经顺序）：阶段名复用设置页签词表，
/// 说明为本组件文案；新增阶段时同步设置页签与该列表。
const steps = [
  { labelKey: 'settings.dataSource', descKey: 'settings.pipelineFlow.steps.dataSource' },
  { labelKey: 'settings.contentProcessor', descKey: 'settings.pipelineFlow.steps.contentProcessor' },
  { labelKey: 'settings.keywordInjector', descKey: 'settings.pipelineFlow.steps.keywordInjector' },
  { labelKey: 'settings.biasRule', descKey: 'settings.pipelineFlow.steps.biasRule' },
  { labelKey: 'settings.exclusionRule', descKey: 'settings.pipelineFlow.steps.exclusionRule' },
  { labelKey: 'settings.searchEngine', descKey: 'settings.pipelineFlow.steps.searchEngine' },
  { labelKey: 'settings.scoreBooster', descKey: 'settings.pipelineFlow.steps.scoreBooster' },
  { labelKey: 'settings.actionExecutor', descKey: 'settings.pipelineFlow.steps.actionExecutor' },
]

const expanded = ref(false)
</script>

<style scoped>
.pipeline-flow-diagram {
  margin-bottom: 24px;
  background-color: var(--bg-color-secondary);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  overflow: hidden;
}

.flow-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 14px 16px;
  cursor: pointer;
  user-select: none;
  transition: background-color 0.2s;
}

.flow-header:hover {
  background-color: var(--hover-color, rgba(128, 128, 128, 0.08));
}

.flow-icon {
  font-size: 10px;
  color: var(--text-color-secondary);
  transition: transform 0.2s ease;
  flex-shrink: 0;
}

.flow-icon.rotated {
  transform: rotate(90deg);
}

.flow-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-color);
}

.flow-steps {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px 16px;
}

.flow-step {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  flex: 1;
}

.step-num {
  width: 24px;
  height: 24px;
  border-radius: 12px;
  background-color: var(--primary-color-alpha);
  color: var(--primary-color);
  font-size: 12px;
  font-weight: 600;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 8px;
}

.step-label {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-color);
  margin-bottom: 4px;
}

.step-desc {
  font-size: 11px;
  color: var(--text-color-secondary);
}

.flow-arrow {
  color: var(--border-color);
  font-size: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  padding-bottom: 16px;
}

/* 折叠展开动画 */
.collapse-enter-active {
  animation: collapse-in 0.25s ease;
}
.collapse-leave-active {
  animation: collapse-out 0.25s ease;
  overflow: hidden;
}

@keyframes collapse-in {
  from {
    max-height: 0;
    opacity: 0;
    padding-block: 0;
  }
  to {
    max-height: 200px;
    opacity: 1;
  }
}

@keyframes collapse-out {
  from {
    max-height: 200px;
    opacity: 1;
  }
  to {
    max-height: 0;
    opacity: 0;
    padding-block: 0;
  }
}
</style>

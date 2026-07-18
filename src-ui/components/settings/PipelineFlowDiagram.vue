<template>
  <div class="pipeline-flow-diagram">
    <!-- 可点击标题栏 -->
    <div class="flow-header" @click="expanded = !expanded">
      <span class="flow-icon" :class="{ rotated: expanded }">▶</span>
      <span class="flow-title">搜索管道处理流程</span>
    </div>

    <!-- 可折叠内容区域 -->
    <Transition name="collapse">
      <div v-if="expanded" class="flow-steps">
        <div class="flow-step">
          <div class="step-num">1</div>
          <div class="step-label">数据源</div>
          <div class="step-desc">提供候选</div>
        </div>
        <div class="flow-arrow">➔</div>
        <div class="flow-step">
          <div class="step-num">2</div>
          <div class="step-label">内容处理器</div>
          <div class="step-desc">处理搜索词</div>
        </div>
        <div class="flow-arrow">➔</div>
        <div class="flow-step">
          <div class="step-num">3</div>
          <div class="step-label">关键字注入器</div>
          <div class="step-desc">注入别名等</div>
        </div>
        <div class="flow-arrow">➔</div>
        <div class="flow-step">
          <div class="step-num">4</div>
          <div class="step-label">固定偏移量</div>
          <div class="step-desc">预设权重</div>
        </div>
        <div class="flow-arrow">➔</div>
        <div class="flow-step">
          <div class="step-num">5</div>
          <div class="step-label">检索引擎</div>
          <div class="step-desc">初筛与算分</div>
        </div>
        <div class="flow-arrow">➔</div>
        <div class="flow-step">
          <div class="step-num">6</div>
          <div class="step-label">评分增强器</div>
          <div class="step-desc">加权与排序</div>
        </div>
        <div class="flow-arrow">➔</div>
        <div class="flow-step">
          <div class="step-num">7</div>
          <div class="step-label">执行器</div>
          <div class="step-desc">回车执行动作</div>
        </div>
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

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

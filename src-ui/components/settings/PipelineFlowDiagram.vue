<template>
  <div class="pipeline-flow-diagram">
    <div class="flow-title">搜索管道处理流程</div>
    <div class="flow-steps">
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
        <div class="step-label">检索引擎</div>
        <div class="step-desc">初筛与算分</div>
      </div>
      <div class="flow-arrow">➔</div>
      <div class="flow-step">
        <div class="step-num">4</div>
        <div class="step-label">评分增强器</div>
        <div class="step-desc">加权与排序</div>
      </div>
      <div class="flow-arrow">➔</div>
      <div class="flow-step">
        <div class="step-num">5</div>
        <div class="step-label">执行器</div>
        <div class="step-desc">回车执行动作</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
</script>

<style scoped>
.pipeline-flow-diagram {
  margin-bottom: 24px;
  padding: 16px;
  background-color: var(--bg-color-secondary);
  border: 1px solid var(--border-color);
  border-radius: 8px;
}

.flow-title {
  font-size: 14px;
  font-weight: 600;
  margin-bottom: 16px;
  color: var(--text-color);
}

.flow-steps {
  display: flex;
  align-items: center;
  justify-content: space-between;
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
</style>

<template>
  <div class="form-section">
    <n-card v-if="hasTitle" size="small" class="section-card">
      <template #header>
        <span class="section-title">{{ title }}</span>
      </template>
      <template v-if="collapsible" #header-extra>
        <n-button text size="small" @click="collapsed = !collapsed">
          {{ collapsed ? '展开' : '收起' }}
        </n-button>
      </template>
      <div v-show="!collapsed" class="section-fields">
        <slot />
      </div>
    </n-card>
    <div v-else class="section-fields">
      <slot />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { NCard, NButton } from 'naive-ui'

const props = withDefaults(
  defineProps<{
    title: string
    collapsible?: boolean
  }>(),
  {
    collapsible: true,
  },
)

const hasTitle = computed(() => props.title !== '')
const collapsed = ref(false)
</script>

<style scoped>
.form-section {
  margin-bottom: 8px;
}

.section-card {
  --n-padding-top: 12px;
  --n-padding-bottom: 12px;
}

.section-title {
  font-size: var(--font-size-base);
  font-weight: 600;
}

.section-fields {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
</style>

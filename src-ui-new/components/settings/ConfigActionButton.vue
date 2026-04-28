<template>
  <n-button
    v-if="actionDef"
    size="small"
    :loading="loading"
    :disabled="!editable"
    @click="executeAction"
  >
    {{ actionDef.label }}
  </n-button>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { NButton } from 'naive-ui'
import { useConfigStore } from '../../stores/config-store'
import type { ConfigActionDef } from '../../bridge/contract'

const props = defineProps<{
  componentId: string
  configAction: string
  fieldKey: string
  editable: boolean
  modelValue: unknown
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()

const configStore = useConfigStore()
const loading = ref(false)
const actionDef = ref<ConfigActionDef | null>(null)

onMounted(async () => {
  try {
    const actions = await configStore.getActions(props.componentId)
    actionDef.value =
      actions.find((a) => a.action === props.configAction) ?? null
  } catch {
    // No actions available
  }
})

async function executeAction() {
  if (!actionDef.value) return

  loading.value = true
  try {
    const result = await configStore.executeAction(
      props.componentId,
      actionDef.value.action,
    )

    if (result && typeof result === 'object') {
      const fieldValue = (result as Record<string, unknown>)[props.fieldKey]
      if (fieldValue !== undefined) {
        emit('update:modelValue', fieldValue)
      }
    }
  } finally {
    loading.value = false
  }
}
</script>

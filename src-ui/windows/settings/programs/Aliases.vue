<template>
    <div class="settings-page">
        <h2 class="page-title">{{ t('program_index.setting_alias') }}</h2>
        <div class="content-container">
            <el-button class="refresh-btn" @click="refreshProgramInfo">
                {{ t('program_index.click_refresh') }}
            </el-button>
            <el-table-v2 :columns="columns" :data="programInfoList" :width="1000" :height="600" fixed
                style="width: 100%; flex-grow: 1; margin-top: 10px;" />
        </div>

        <el-dialog v-if="editingProgram" v-model="dialogVisible"
            :title="t('settings.edit_program_alias', { name: editingProgram.name })" width="500">
            <div style="display: flex; flex-direction: column; gap: 10px;">
                <div v-for="(alias, index) in program_alias[editingProgram.path]" :key="index"
                    style="display: flex; align-items: center; gap: 10px;">
                    <el-input :model-value="alias" @update:modelValue="(newValue: string) => updateAliasInDialog(index, newValue)"
                        :placeholder="t('settings.enter_alias')" />
                    <el-button type="danger" @click="removeAliasInDialog(index)">{{ t('settings.delete') }}</el-button>
                </div>
            </div>
            <template #footer>
                <div class="dialog-footer">
                    <el-button @click="addAliasInDialog" style="width: 100%; margin-bottom: 10px;">{{
                        t('settings.add_alias') }}</el-button>
                    <el-button type="primary" @click="dialogVisible = false">{{ t('settings.close') }}</el-button>
                </div>
            </template>
        </el-dialog>
    </div>
</template>

<script lang="ts" setup>
import { computed, ref, h } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRemoteConfigStore } from '../../../stores/remote_config';
import { storeToRefs } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import { ElButton, ElTag } from 'element-plus';

const { t } = useI18n();
const configStore = useRemoteConfigStore();
const { config } = storeToRefs(configStore);

interface ProgramInfo {
    name: string
    is_uwp: boolean
    bias: number
    history_launch_time: number
    path: string
}

const programInfoList = ref<ProgramInfo[]>([])
const dialogVisible = ref(false)
const editingProgram = ref<ProgramInfo | null>(null)

const program_alias = computed({
    get: () => config.value.program_manager_config.loader.program_alias,
    set: (value) => {
        configStore.updateConfig({
            program_manager_config: {
                loader: { program_alias: value }
            }
        })
    }
})

const columns = computed(() => [
    { key: 'name', dataKey: 'name', title: t('settings.program_name'), width: 150 },
    { key: 'is_uwp', dataKey: 'is_uwp', title: t('settings.is_uwp_program'), width: 120 },
    { key: 'bias', dataKey: 'bias', title: t('settings.fixed_offset'), width: 100 },
    { key: 'history_launch_time', dataKey: 'history_launch_time', title: t('settings.launch_count'), width: 100 },
    { key: 'path', dataKey: 'path', title: t('settings.path'), width: 200 },
    {
        key: 'aliases',
        title: t('settings.aliases'),
        width: 300,
        cellRenderer: ({ rowData }: { rowData: ProgramInfo }) => {
            const aliasList = program_alias.value[rowData.path] || [];
            const tags = aliasList.map(alias =>
                h(ElTag, { style: 'margin-right: 5px; margin-bottom: 5px;', type: 'info', size: 'small' }, () => alias)
            );
            const editButton = h(ElButton, {
                size: 'small',
                type: 'primary',
                link: true,
                onClick: () => handleEditAliases(rowData)
            }, () => t('settings.manage_aliases'));
            return h('div', { style: 'display: flex; flex-wrap: wrap; align-items: center;' }, [...tags, editButton]);
        }
    }
]);

const refreshProgramInfo = async () => {
    try {
        const data = await invoke<ProgramInfo[]>('get_program_info')
        programInfoList.value = data
    } catch (error) {
        console.error(t('settings.get_program_info_failed'), error)
    }
}

const handleEditAliases = (rowData: ProgramInfo) => {
    editingProgram.value = { ...rowData };
    dialogVisible.value = true;
}

const addAliasInDialog = () => {
    if (!editingProgram.value) return;
    const path = editingProgram.value.path;
    const currentAliases = program_alias.value[path] || [];
    const newAliases = { ...program_alias.value };
    newAliases[path] = [...currentAliases, ""];
    program_alias.value = newAliases;
}

const updateAliasInDialog = (index: number, newValue: string) => {
    if (!editingProgram.value) return;
    const path = editingProgram.value.path;
    const newProgramAlias = { ...program_alias.value };
    const currentAliases = [...(newProgramAlias[path] || [])];
    if (index >= 0 && index < currentAliases.length) {
        currentAliases[index] = newValue;
    }
    newProgramAlias[path] = currentAliases;
    program_alias.value = newProgramAlias;
}

const removeAliasInDialog = (index: number) => {
    if (!editingProgram.value) return;
    const path = editingProgram.value.path;
    const currentAliases = program_alias.value[path] || [];
    if (index >= 0 && index < currentAliases.length) {
        const newAliases = { ...program_alias.value };
        newAliases[path] = currentAliases.filter((_, i) => i !== index);
        if (newAliases[path].length === 0) {
            delete newAliases[path];
        }
        program_alias.value = newAliases;
    }
}
</script>

<style scoped>
.settings-page {
    padding: 20px;
    height: 100%;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
}

.page-title {
    margin-top: 0;
    margin-bottom: 20px;
    font-size: 20px;
    font-weight: 500;
    color: #303133;
}

.content-container {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
}

.refresh-btn {
    width: 100%;
    flex-shrink: 0;
}
</style>

<template>
    <div class="settings-page">
        <h2 class="page-title">{{ t('program_index.set_blocked_paths') }}</h2>
        <div class="content-container">
            <el-button class="add-btn" @click="addForbiddenPath">
                <el-icon><Plus /></el-icon> {{ t('program_index.add_item') }}
            </el-button>
            <el-table :data="forbidden_paths" stripe style="width: 100%; margin-top: 10px;">
                <el-table-column :label="t('program_index.target_blocked_path')" show-overflow-tooltip>
                    <template #default="{ $index }">
                        <el-input v-model="forbidden_paths[$index]" size="small"
                            :placeholder="t('program_index.enter_target_path')"
                            @change="updateForbiddenPaths"></el-input>
                    </template>
                </el-table-column>
                <el-table-column fixed="right" :label="t('program_index.operation')" width="100">
                    <template #default="{ $index }">
                        <el-button link size="small" type="danger" @click="deleteForbiddenPath($index)">
                            {{ t('program_index.delete_row') }}
                        </el-button>
                    </template>
                </el-table-column>
            </el-table>
        </div>
    </div>
</template>

<script lang="ts" setup>
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRemoteConfigStore } from '../../../stores/remote_config';
import { storeToRefs } from 'pinia';
import { Plus } from '@element-plus/icons-vue';

const { t } = useI18n();
const configStore = useRemoteConfigStore();
const { config } = storeToRefs(configStore);

const forbidden_paths = computed({
    get: () => config.value.program_manager_config.loader.forbidden_paths,
    set: (value) => {
        configStore.updateConfig({
            program_manager_config: {
                loader: { forbidden_paths: value }
            }
        })
    }
})

const updateForbiddenPaths = () => {
    configStore.updateConfig({
        program_manager_config: {
            loader: { forbidden_paths: forbidden_paths.value }
        }
    })
}

const deleteForbiddenPath = (index: number) => {
    const newPaths = [...forbidden_paths.value]
    newPaths.splice(index, 1)
    forbidden_paths.value = newPaths
}

const addForbiddenPath = () => {
    forbidden_paths.value = [...forbidden_paths.value, ""]
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
    overflow-y: auto;
}

.add-btn {
    width: 100%;
    flex-shrink: 0;
}
</style>

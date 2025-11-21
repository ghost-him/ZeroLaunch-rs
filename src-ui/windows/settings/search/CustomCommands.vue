<template>
    <div class="settings-page">
        <h2 class="page-title">{{ t('settings.custom_command_search') }}</h2>
        <div class="content-container">
            <el-button class="add-btn" @click="addCustomCommand">
                <el-icon><Plus /></el-icon> {{ t('settings.add_item') }}
            </el-button>
            <el-table :data="custom_command" stripe style="width: 100%; margin-top: 10px;">
                <el-table-column :label="t('settings.keyword_for_search')" show-overflow-tooltip fixed="left" width="150">
                    <template #default="scope">
                        <el-input v-model="custom_command[scope.$index][0]" size="small"
                            :placeholder="t('settings.enter_keyword')"
                            @change="updateCustomCommand"></el-input>
                    </template>
                </el-table-column>
                <el-table-column :label="t('settings.command_content')" show-overflow-tooltip>
                    <template #default="scope">
                        <el-input v-model="custom_command[scope.$index][1]" size="small"
                            :placeholder="t('settings.enter_command_content')"
                            @change="updateCustomCommand"></el-input>
                    </template>
                </el-table-column>
                <el-table-column fixed="right" :label="t('settings.actions')" width="100">
                    <template #default="scope">
                        <el-button link size="small" type="danger" @click="deleteCustomCommand(scope.$index)">
                            {{ t('settings.delete_row') }}
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

const custom_command = computed({
    get: () => config.value.program_manager_config.loader.custom_command,
    set: (value) => {
        configStore.updateConfig({
            program_manager_config: {
                loader: { custom_command: value }
            }
        })
    }
})

const deleteCustomCommand = (index: number) => {
    custom_command.value = custom_command.value.filter((_, i) => i !== index)
}

const updateCustomCommand = () => {
    configStore.updateConfig({
        program_manager_config: {
            loader: { custom_command: custom_command.value }
        }
    })
}

const addCustomCommand = () => {
    custom_command.value = [...custom_command.value, ["", ""]]
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

.add-btn {
    width: 100%;
    flex-shrink: 0;
}
</style>

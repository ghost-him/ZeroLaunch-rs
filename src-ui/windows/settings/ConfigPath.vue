<template>
    <div class="settings-page">
        <h2 class="page-title">{{ t('config_path.title') }}</h2>
        <div class="content-container">
            <el-form :model="formData" label-width="180px" class="storage-config-form">
            <el-form-item :label="t('config_path.storage_location')" class="storage-type-selector">
                <el-radio-group v-model="formData.storage_destination" size="large">
                    <el-radio-button value="Local">{{ t('config_path.local_storage') }}</el-radio-button>
                    <el-radio-button value="WebDAV">WebDAV</el-radio-button>
                    <!--<el-radio-button value="OneDrive">OneDrive</el-radio-button> -->
                </el-radio-group>
            </el-form-item>

            <!-- 存储配置区块 -->
            <div class="config-section">
                <div v-if="formData.storage_destination === 'Local'" class="storage-section">
                    <h3 class="section-title">{{ t('config_path.local_storage_settings') }}</h3>
                    <el-form-item :label="t('config_path.config_file_save_path')">
                        <div class="path-input-container">
                            <el-input v-model="formData.local_save_config.destination_dir"
                                :placeholder="t('config_path.config_file_save_path_placeholder')" readonly
                                class="path-display" :title="formData.local_save_config.destination_dir">
                                <template #prefix>
                                    <el-icon>
                                        <Folder />
                                    </el-icon>
                                </template>
                            </el-input>
                            <div class="path-buttons">
                                <el-button type="primary" @click="handleChangeConfigPath">
                                    <el-icon>
                                        <FolderOpened />
                                    </el-icon> {{ t('config_path.select_path') }}
                                </el-button>
                                <el-button @click="handleUseDefaultPath">
                                    <el-icon>
                                        <SetUp />
                                    </el-icon> {{ t('config_path.use_default_path') }}
                                </el-button>
                            </div>
                        </div>
                    </el-form-item>
                </div>
            </div>

            <!-- WebDAV 配置 -->
            <div v-if="formData.storage_destination === 'WebDAV'" class="storage-section">
                <h3 class="section-title">{{ t('config_path.webdav_settings') }}</h3>
                <el-form-item :label="t('config_path.server_address')">
                    <el-input v-model="formData.webdav_save_config.host_url"
                        :placeholder="t('config_path.server_address_placeholder')">
                        <template #prefix>
                            <el-icon>
                                <Link />
                            </el-icon>
                        </template>
                    </el-input>
                </el-form-item>
                <el-form-item :label="t('config_path.account')">
                    <el-input v-model="formData.webdav_save_config.account"
                        :placeholder="t('config_path.account_placeholder')">
                        <template #prefix>
                            <el-icon>
                                <User />
                            </el-icon>
                        </template>
                    </el-input>
                </el-form-item>
                <el-form-item :label="t('config_path.password')">
                    <el-input v-model="formData.webdav_save_config.password" type="password"
                        :placeholder="t('config_path.password_placeholder')" show-password>
                        <template #prefix>
                            <el-icon>
                                <Lock />
                            </el-icon>
                        </template>
                    </el-input>
                </el-form-item>
                <el-form-item :label="t('config_path.target_directory')">
                    <el-input v-model="formData.webdav_save_config.destination_dir"
                        :placeholder="t('config_path.target_directory_placeholder')">
                        <template #prefix>
                            <el-icon>
                                <FolderOpened />
                            </el-icon>
                        </template>
                    </el-input>
                </el-form-item>
            </div>

            <!-- 通用设置区块 -->
            <div class="config-section">
                <h3 class="section-title">{{ t('config_path.general_settings') }}</h3>
                <el-form-item :label="t('config_path.config_cache_count')">
                    <el-input-number v-model="formData.save_to_local_per_update" :min="0" :step="1" :precision="0"
                        controls-position="right" class="number-input" />
                    <span class="input-description">{{ t('config_path.config_cache_description') }}</span>
                </el-form-item>
            </div>

            <div class="action-buttons">
                <el-button type="primary" @click="testConfigValidation">
                    <el-icon>
                        <Connection />
                    </el-icon> {{ t('config_path.test_connection') }}
                </el-button>
                <el-button type="primary" @click="saveConfig" :disabled="!allowSave">
                    <el-icon>
                        <Collection />
                    </el-icon> {{ t('config_path.save_config') }}
                </el-button>
                <el-button @click="resetConfig">
                    <el-icon>
                        <RefreshRight />
                    </el-icon> {{ t('config_path.reset') }}
                </el-button>
            </div>
            </el-form>
        </div>
    </div>
</template>

<script lang="ts" setup>
import { ref, reactive, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useLocalConfigStore } from '../../stores/local_config'
import { ElMessage } from 'element-plus'
import {
    Folder, FolderOpened, SetUp, Link, User, Lock,
    Connection, Collection, RefreshRight
} from '@element-plus/icons-vue'
import { open } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'
import { PartialLocalConfig } from '../../api/local_config_types'
import { listen, UnlistenFn } from '@tauri-apps/api/event'

const { t } = useI18n()
const auth_link = ref('')
const allowSave = ref(false)
// 获取配置存储
const configStore = useLocalConfigStore()

// 表单数据
const formData = reactive({
    storage_destination: configStore.config.storage_destination,
    local_save_config: { ...configStore.config.local_save_config },
    webdav_save_config: { ...configStore.config.webdav_save_config },
    // onedrive_save_config: { ...configStore.config.onedrive_save_config },
    save_to_local_per_update: configStore.config.save_to_local_per_update
})

watch(
    () => formData,
    () => {
        allowSave.value = false
    },
    { deep: true }
)
let unlisten: Array<UnlistenFn | null> = [];
// 初始化
onMounted(async () => {
    await configStore.loadConfig();
    Object.assign(formData, configStore.config);
    unlisten.push(await listen('emit_update_auth_link', async (event) => {
        auth_link.value = event.payload as string;
    }))
})

// 选择本地配置文件路径
const handleChangeConfigPath = async () => {
    try {
        const selected = await open({
            canCreateDirectories: true,
            directory: true,
            multiple: false,
            title: t('config_path.select_config_save_location')
        });

        if (selected) {
            formData.local_save_config.destination_dir = selected;
        }
    } catch (error) {
        handleError(t('config_path.select_folder_failed'), error);
    }


}

// 使用默认路径
const handleUseDefaultPath = async () => {
    const default_path = await invoke<string>('command_get_default_remote_data_dir_path');
    formData.local_save_config.destination_dir = default_path
    ElMessage.success(t('config_path.set_default_path_success'))
}

const testConfigValidation = async () => {
    try {
        const validation = await invoke<PartialLocalConfig>('command_check_validation', { partialConfig: formData });

        if (validation) {
            ElMessage.success(t('config_path.connection_success'))
            allowSave.value = true // 测试成功后允许保存
            configStore.updateConfig(validation)
        } else {
            ElMessage.error(t('config_path.connection_failed'))
            allowSave.value = false
        }

    } catch (error) {
        ElMessage.error(t('config_path.connection_failed'))
        allowSave.value = false
    }
}

// 保存配置
const saveConfig = async () => {
    try {
        await configStore.updateConfig(formData)
        configStore.syncConfig();
        ElMessage.success(t('config_path.config_saved'))
    } catch (error) {
        handleError(t('config_path.save_config_failed'), error)
    }
}

// 重置配置
const resetConfig = () => {
    Object.assign(formData, {
        storage_destination: configStore.config.storage_destination,
        local_save_config: { ...configStore.config.local_save_config },
        webdav_save_config: { ...configStore.config.webdav_save_config },
        // onedrive_save_config: { ...configStore.config.onedrive_save_config },
        save_to_local_per_update: configStore.config.save_to_local_per_update
    })
    auth_link.value = ''
    ElMessage.info(t('config_path.config_reset'))
}

const handleError = (message: string, error: unknown) => {
    console.error(message, error)
    ElMessage.error(message)
};

</script>

<style scoped>
.settings-page {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 20px;
    box-sizing: border-box;
}

.page-title {
    margin-top: 0;
    margin-bottom: 20px;
    font-size: 24px;
    font-weight: 500;
    color: var(--el-text-color-primary);
    flex-shrink: 0;
}

.content-container {
    flex: 1;
    overflow-y: auto;
    /* Add padding to avoid content being cut off by scrollbar */
    padding-right: 10px;
}

.storage-section {
    background-color: var(--el-fill-color-light);
    border-radius: 8px;
    padding: 20px;
    margin-bottom: 24px;
    box-shadow: 0 2px 12px 0 rgba(0, 0, 0, 0.05);
}

.section-title {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 20px;
    color: var(--el-text-color-primary);
    font-weight: 500;
}

.input-description {
    margin-left: 12px;
    color: var(--el-text-color-secondary);
    font-size: 14px;
}

.action-buttons {
    display: flex;
    gap: 12px;
    margin-top: 24px;
    justify-content: flex-start;
}


.config-section {
    margin-top: 24px;
    padding-top: 24px;
    border-top: 1px solid var(--el-border-color-light);
}

.storage-config-form {
    max-width: 100%;
}

.storage-type-selector {
    margin-bottom: 25px;
    display: flex;
    justify-content: center;
}

/* 保持其他原有样式不变 */
.path-input-container {
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: 100%;
}

.path-display {
    font-family: 'Courier New', monospace;
    font-size: 14px;
    width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.path-buttons {
    display: flex;
    gap: 10px;
}

.number-input {
    width: 180px;
}

.input-description {
    margin-left: 15px;
    color: #909399;
    font-size: 14px;
}

.action-buttons {
    display: flex;
    justify-content: center;
    gap: 20px;
    margin-top: 30px;
    margin-bottom: 30px;
}

.path-display :deep(.el-input__wrapper) {
    padding-right: 15px;
}

.path-display :deep(.el-input__inner) {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
</style>

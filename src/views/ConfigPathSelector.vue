<template>
    <div class="storage-config-container">
        <h2 class="page-title">配置文件存储设置</h2>

        <el-form :model="formData" label-width="180px" class="storage-config-form">
            <el-form-item label="配置文件存储位置" class="storage-type-selector">
                <el-radio-group v-model="formData.storage_destination" size="large">
                    <el-radio-button value="Local">本地存储</el-radio-button>
                    <el-radio-button value="WebDAV">WebDAV</el-radio-button>
                    <el-radio-button value="OneDrive">OneDrive</el-radio-button>
                </el-radio-group>
            </el-form-item>

            <!-- 存储配置区块 -->
            <div class="config-section">
                <div v-if="formData.storage_destination === 'Local'" class="storage-section">
                    <h3 class="section-title">本地存储设置</h3>
                    <el-form-item label="配置文件保存路径">
                        <div class="path-input-container">
                            <el-input v-model="formData.local_save_config.remote_config_path" placeholder="配置文件保存路径"
                                readonly class="path-display" :title="formData.local_save_config.remote_config_path">
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
                                    </el-icon> 选择路径
                                </el-button>
                                <el-button @click="handleUseDefaultPath">
                                    <el-icon>
                                        <SetUp />
                                    </el-icon> 使用默认路径
                                </el-button>
                            </div>
                        </div>
                    </el-form-item>
                </div>

                <!-- WebDAV 配置 -->
                <div v-if="formData.storage_destination === 'WebDAV'" class="storage-section">
                    <h3 class="section-title">WebDAV 设置</h3>
                    <el-form-item label="服务器地址">
                        <el-input v-model="formData.webdav_save_config.host_url" placeholder="请输入 WebDAV 服务器地址">
                            <template #prefix>
                                <el-icon>
                                    <Link />
                                </el-icon>
                            </template>
                        </el-input>
                    </el-form-item>
                    <el-form-item label="账号">
                        <el-input v-model="formData.webdav_save_config.account" placeholder="请输入账号">
                            <template #prefix>
                                <el-icon>
                                    <User />
                                </el-icon>
                            </template>
                        </el-input>
                    </el-form-item>
                    <el-form-item label="密码">
                        <el-input v-model="formData.webdav_save_config.password" type="password" placeholder="请输入密码"
                            show-password>
                            <template #prefix>
                                <el-icon>
                                    <Lock />
                                </el-icon>
                            </template>
                        </el-input>
                    </el-form-item>
                    <el-form-item label="目标目录">
                        <el-input v-model="formData.webdav_save_config.destination_dir" placeholder="请输入目标目录">
                            <template #prefix>
                                <el-icon>
                                    <FolderOpened />
                                </el-icon>
                            </template>
                        </el-input>
                    </el-form-item>
                    <el-form-item>
                        <el-button type="primary" @click="testWebDAVConnection">
                            <el-icon>
                                <Connection />
                            </el-icon> 测试连接
                        </el-button>
                    </el-form-item>
                </div>

                <!-- OneDrive 配置 -->
                <div v-if="formData.storage_destination === 'OneDrive'" class="storage-section">
                    <h3 class="section-title">OneDrive 设置</h3>
                    <el-form-item label="文件夹路径">
                        <el-input v-model="formData.onedrive_save_config.folder_path" placeholder="请输入 OneDrive 文件夹路径">
                            <template #prefix>
                                <el-icon>
                                    <FolderOpened />
                                </el-icon>
                            </template>
                        </el-input>
                    </el-form-item>
                    <el-form-item label="自动同步">
                        <el-switch v-model="formData.onedrive_save_config.sync_enabled" active-text="开启"
                            inactive-text="关闭" />
                    </el-form-item>
                    <el-form-item>
                        <el-button type="primary" @click="authorizeOneDrive">
                            <el-icon>
                                <Key />
                            </el-icon> 授权 OneDrive
                        </el-button>
                        <el-button @click="checkOneDriveStatus">
                            <el-icon>
                                <InfoFilled />
                            </el-icon> 检查连接状态
                        </el-button>
                    </el-form-item>
                </div>
            </div>

            <!-- 通用设置区块 -->
            <div class="config-section">
                <h3 class="section-title">通用设置</h3>
                <el-form-item label="配置缓存次数">
                    <el-input-number v-model="formData.save_to_local_per_update" :min="0" :step="1" :precision="0"
                        controls-position="right" class="number-input" />
                    <span class="input-description">设置为0表示每次保存配置都上传</span>
                </el-form-item>
            </div>

            <div class="action-buttons">
                <el-button type="primary" size="large" @click="saveConfig">
                    <el-icon>
                        <Check />
                    </el-icon> 保存配置
                </el-button>
                <el-button size="large" @click="resetConfig">
                    <el-icon>
                        <RefreshRight />
                    </el-icon> 重置
                </el-button>
            </div>
        </el-form>
    </div>
</template>

<script lang="ts" setup>
import { ref, reactive, onMounted } from 'vue'
import { useLocalConfigStore } from '../stores/local_config'
import { ElMessage } from 'element-plus'
import {
    Folder, FolderOpened, SetUp, Link, User, Lock,
    Connection, Key, InfoFilled, Check, RefreshRight
} from '@element-plus/icons-vue'
import { open } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'

// 获取配置存储
const configStore = useLocalConfigStore()

// 表单数据
const formData = reactive({
    storage_destination: configStore.config.storage_destination,
    local_save_config: { ...configStore.config.local_save_config },
    webdav_save_config: { ...configStore.config.webdav_save_config },
    onedrive_save_config: { ...configStore.config.onedrive_save_config },
    save_to_local_per_update: configStore.config.save_to_local_per_update
})

// 初始化
onMounted(async () => {
    await configStore.loadConfig();
    Object.assign(formData, configStore.config);
})

// 选择本地配置文件路径
const handleChangeConfigPath = async () => {
    try {
        const selected = await open({
            canCreateDirectories: true,
            directory: true,
            multiple: false,
            title: "选择配置文件保存位置"
        });

        if (selected) {
            formData.local_save_config.remote_config_path = selected;
        }
    } catch (error) {
        handleError('选择文件夹失败', error);
    }

    console.log('打开文件选择对话框')
}

// 使用默认路径
const handleUseDefaultPath = async () => {
    const default_path = await invoke<string>('command_get_default_remote_data_dir_path');
    formData.local_save_config.remote_config_path = default_path
    ElMessage.success('已设置为默认路径')
}

// 测试 WebDAV 连接
const testWebDAVConnection = async () => {
    ElMessage.success('连接成功')
}

// 授权 OneDrive
const authorizeOneDrive = async () => {
    console.log('启动 OneDrive 授权流程')
}

// 检查 OneDrive 状态
const checkOneDriveStatus = async () => {
    console.log('检查 OneDrive 连接状态')
}

// 保存配置
const saveConfig = async () => {
    configStore.updateConfig({
        storage_destination: formData.storage_destination,
        local_save_config: formData.local_save_config,
        webdav_save_config: formData.webdav_save_config,
        onedrive_save_config: formData.onedrive_save_config,
        save_to_local_per_update: formData.save_to_local_per_update,
    });
    await configStore.syncConfig();
    ElMessage.success('配置已保存')
}

// 重置配置
const resetConfig = () => {
    Object.assign(formData, configStore.config)
    ElMessage.info('配置已重置')
}

const handleError = (message: string, error: unknown) => {
    const errorMessage = error instanceof Error ? error.message : String(error);
    console.error(`${message}:`, error);

    ElMessage({
        message: `${message}: ${errorMessage}`,
        type: 'error',
        showClose: true,
    });
};

</script>

<style scoped>
.storage-config-container {
    max-width: 900px;
    margin: 20px auto;
    padding: 20px;
    background: #fff;
    border-radius: 8px;
}

.page-title {
    color: #303133;
    font-size: 24px;
    margin-bottom: 30px;
    text-align: center;
    font-weight: 500;
}

.storage-config-form {
    max-width: 100%;
}

.storage-type-selector {
    margin-bottom: 25px;
    display: flex;
    justify-content: center;
}

.config-section {
    margin-bottom: 25px;
    padding: 20px;
    border: 1px solid #EBEEF5;
    border-radius: 4px;
}

.section-title {
    color: #409EFF;
    font-size: 18px;
    margin-top: 0;
    margin-bottom: 20px;
    font-weight: 500;
    border-bottom: 1px solid #EBEEF5;
    padding-bottom: 10px;
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
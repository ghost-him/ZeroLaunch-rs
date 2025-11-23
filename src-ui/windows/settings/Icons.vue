<template>
    <div class="settings-page">
        <h2 class="page-title">
            {{ t('settings.icon_management') }}
        </h2>
        <div class="content-container">
            <el-form label-width="auto" class="settings-form">
                <el-form-item :label="t('icon_management.enable_online_icon_loading')">
                    <el-switch v-model="config.icon_manager_config.enable_online" @change="(val: boolean) =>
                        configStore.updateConfig({
                            icon_manager_config: { enable_online: val }
                        })
                    " />
                    <el-tooltip class="box-item" effect="dark" :content="t('icon_management.online_icon_tooltip')">
                        <el-icon class="el-question-icon">
                            <QuestionFilled />
                        </el-icon>
                    </el-tooltip>
                </el-form-item>

                <el-form-item :label="t('icon_management.enable_icon_cache')">
                    <el-switch v-model="config.icon_manager_config.enable_icon_cache" @change="(val: boolean) =>
                        configStore.updateConfig({
                            icon_manager_config: { enable_icon_cache: val }
                        })
                    " />
                    <el-tooltip class="box-item" effect="dark" :content="t('icon_management.icon_cache_tooltip')">
                        <el-icon class="el-question-icon">
                            <QuestionFilled />
                        </el-icon>
                    </el-tooltip>
                </el-form-item>

                <el-form-item>
                    <el-button type="primary" @click="openIconCacheDir">
                        {{ t('icon_management.open_icon_cache_folder') }}
                    </el-button>
                </el-form-item>
            </el-form>

            <el-divider />

            <div class="custom-icon-section">
                <h3>
                    {{ t('icon_management.custom_program_icon') }}
                    <el-tooltip effect="dark" :content="t('icon_management.refresh_note')">
                        <el-icon class="el-question-icon">
                            <QuestionFilled />
                        </el-icon>
                    </el-tooltip>
                </h3>

                <el-alert v-if="!config.icon_manager_config.enable_icon_cache"
                    :title="t('icon_management.icon_cache_disabled_warning')" type="warning" show-icon :closable="false"
                    style="margin-bottom: 16px;" />

                <div v-else class="table-container">
                    <el-input v-model="searchKeyword" :placeholder="t('icon_management.search_placeholder')"
                        prefix-icon="Search" clearable @input="handleSearch" style="margin-bottom: 16px;" />

                    <div class="table-wrapper">
                        <el-table v-loading="loading" :data="programList" style="width: 100%" height="100%">
                        <el-table-column :label="t('icon_management.icon')" width="80">
                            <template #default="scope">
                                <img :src="getIconUrl(scope.row.icon_request_json)" class="program-icon" alt="icon" />
                            </template>
                        </el-table-column>

                        <el-table-column :label="t('icon_management.program_name')" prop="name" width="200" />

                        <el-table-column :label="t('icon_management.path')" prop="path">
                            <template #default="scope">
                                <span class="path-text">{{ scope.row.path }}</span>
                            </template>
                        </el-table-column>

                        <el-table-column :label="t('icon_management.actions')" width="120" fixed="right">
                            <template #default="scope">
                                <el-button size="small" type="primary" @click="handleChangeIcon(scope.row)">
                                    {{ t('icon_management.change_icon') }}
                                </el-button>
                            </template>
                        </el-table-column>
                    </el-table>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { storeToRefs } from 'pinia'
import { useRemoteConfigStore } from '../../stores/remote_config'
import { useI18n } from 'vue-i18n'
import { QuestionFilled } from '@element-plus/icons-vue'
import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { ElMessage } from 'element-plus'

const { t } = useI18n()
const configStore = useRemoteConfigStore()
const { config } = storeToRefs(configStore)

type ProgramIconEntry = {
    name: string    // 显示的名字
    path: string    // 唯一标识符，用于让用户知道具体是哪个程序
    program_guid: number
    icon_request_json: string
}

const searchKeyword = ref('')
const loading = ref(false)
const programList = ref<ProgramIconEntry[]>([])
const iconUrls = ref(new Map<string, string>())

let searchTimeout: number | undefined

const handleSearch = () => {
    if (searchTimeout) clearTimeout(searchTimeout)
    searchTimeout = window.setTimeout(async () => {
        loading.value = true
        try {
            const results = await invoke<ProgramIconEntry[]>('command_search_programs_for_icon_edit', {
                keyword: searchKeyword.value
            })
            programList.value = results
            // Load icons for results
            results.forEach(loadIcon)
        } catch (e) {
            console.error('Search failed', e)
        } finally {
            loading.value = false
        }
    }, 300)
}

const loadIcon = async (row: ProgramIconEntry) => {
    if (iconUrls.value.has(row.icon_request_json)) return
    try {
        const data = await invoke<number[]>('load_program_icon', { programGuid: row.program_guid })

        // Optimize: Process in chunks to avoid Maximum call stack size exceeded
        const bytes = new Uint8Array(data)
        let binary = ''
        const len = bytes.byteLength
        const chunkSize = 0x8000 // 32KB
        for (let i = 0; i < len; i += chunkSize) {
            const chunk = bytes.subarray(i, Math.min(i + chunkSize, len))
            binary += String.fromCharCode.apply(null, chunk as any)
        }
        const base64 = btoa(binary)

        iconUrls.value.set(row.icon_request_json, `data:image/png;base64,${base64}`)
    } catch (e) {
        console.error('Failed to load icon', e)
    }
}

const getIconUrl = (icon_request_json: string) => {
    return iconUrls.value.get(icon_request_json) || ''
}

const handleChangeIcon = async (program: ProgramIconEntry) => {
    try {
        const selected = await open({
            multiple: false,
            filters: [{
                name: 'Image or Executable',
                extensions: ['png', 'jpg', 'jpeg', 'ico', 'svg', 'exe', 'lnk']
            }]
        })

        if (selected && typeof selected === 'string') {
            await invoke('command_update_program_icon', {
                iconRequestJson: program.icon_request_json,
                newIconPath: selected
            })

            // Force refresh icon
            iconUrls.value.delete(program.icon_request_json)
            await loadIcon(program)

            ElMessage.success(t('icon_management.update_success'))
        }
    } catch (e) {
        console.error('Failed to update icon', e)
        ElMessage.error(t('icon_management.update_failed') + `: ${e}`)
    }
}

const openIconCacheDir = async () => {
    try {
        await invoke('command_open_icon_cache_dir')
    } catch (e) {
        console.error('Failed to open icon cache dir', e)
    }
}

// Initial load
handleSearch()
</script>

<style scoped>
.settings-page {
    padding: 20px;
    height: 100%;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
}

.content-container {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    overflow-x: hidden;
}

.page-title {
    margin-top: 0;
    margin-bottom: 20px;
    font-size: 20px;
    font-weight: 500;
    color: #303133;
}

.settings-form {
    max-width: 800px;
    flex-shrink: 0;
}

.el-question-icon {
    margin-left: 8px;
    color: #909399;
    cursor: help;
}

.custom-icon-section {
    margin-top: 20px;
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 400px;
}

.table-container {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
}

.table-wrapper {
    flex: 1;
    min-height: 0;
}

.program-icon {
    width: 32px;
    height: 32px;
    object-fit: contain;
}

.path-text {
    font-size: 12px;
    color: #909399;
    word-break: break-all;
}
</style>

import { defineStore } from 'pinia'
import {
    LocalConfig,
    LocalSaveConfig,
    WebDAVConfig,
    // OneDriveConfig,
    PartialLocalConfig,
    StorageDestination
} from '../api/local_config_types'
import { invoke } from '@tauri-apps/api/core'

// 合并完整配置的辅助函数
function mergeConfig(config: LocalConfig, partial: PartialLocalConfig): LocalConfig {
    return {
        // 保留原配置的所有属性
        ...config,
        // 覆盖顶层基本属性
        ...(partial.storage_destination !== undefined ? { storage_destination: partial.storage_destination } : {}),
        ...(partial.save_to_local_per_update !== undefined ? { save_to_local_per_update: partial.save_to_local_per_update } : {}),
        ...(partial.welcome_page_version !== undefined ? { welcome_page_version: partial.welcome_page_version } : {}),

        // 浅层合并嵌套对象，而不是完全替换
        local_save_config: partial.local_save_config !== undefined
            ? { ...config.local_save_config, ...partial.local_save_config }
            : config.local_save_config,

        webdav_save_config: partial.webdav_save_config !== undefined
            ? { ...config.webdav_save_config, ...partial.webdav_save_config }
            : config.webdav_save_config,

        // onedrive_save_config: partial.onedrive_save_config !== undefined
        //     ? { ...config.onedrive_save_config, ...partial.onedrive_save_config }
        //     : config.onedrive_save_config,
    };
}

// 合并两个 partial 配置的辅助函数
function mergePartialConfig(
    oldPartial: PartialLocalConfig,
    newPartial: PartialLocalConfig
): PartialLocalConfig {
    // 创建结果对象
    const result: PartialLocalConfig = { ...oldPartial };
    
    // 合并顶层基本属性
    if (newPartial.storage_destination !== undefined) {
        result.storage_destination = newPartial.storage_destination;
    }
    
    if (newPartial.save_to_local_per_update !== undefined) {
        result.save_to_local_per_update = newPartial.save_to_local_per_update;
    }
    
    if (newPartial.welcome_page_version !== undefined) {
        result.welcome_page_version = newPartial.welcome_page_version;
    }
    
    // 浅层合并嵌套对象
    if (newPartial.local_save_config !== undefined) {
        result.local_save_config = {
            ...(oldPartial.local_save_config || {}),
            ...newPartial.local_save_config
        };
    }
    
    if (newPartial.webdav_save_config !== undefined) {
        result.webdav_save_config = {
            ...(oldPartial.webdav_save_config || {}),
            ...newPartial.webdav_save_config
        };
    }
    
    // if (newPartial.onedrive_save_config !== undefined) {
    //     result.onedrive_save_config = {
    //         ...(oldPartial.onedrive_save_config || {}),
    //         ...newPartial.onedrive_save_config
    //     };
    // }
    
    return result;
}

export const useLocalConfigStore = defineStore('localConfig', {
    state: () => ({
        config: {
            storage_destination: "Local" as StorageDestination,
            local_save_config: {
                destination_dir: ""
            } as LocalSaveConfig,
            webdav_save_config: {
                host_url: "",
                account: "",
                password: "",
                destination_dir: ""
            } as WebDAVConfig,
            // onedrive_save_config: {
            //     refresh_token: "",
            //     destination_dir: ""
            // } as OneDriveConfig,
            save_to_local_per_update: 0
        } as LocalConfig,
        dirtyConfig: {} as PartialLocalConfig
    }),

    actions: {
        // 从后端加载完整配置
        async loadConfig() {
            try {
                const config = await invoke<PartialLocalConfig>('command_load_local_config')
                this.config = mergeConfig(this.config, config)
            } catch (error) {
                console.error("加载配置失败:", error)
            }
        },

        // 更新配置
        updateConfig(partial: PartialLocalConfig) {
            console.log("收到消息：", partial)
            // 更新合并后的配置
            this.config = mergeConfig(this.config, partial)
            // 合并脏数据配置
            this.dirtyConfig = mergePartialConfig(this.dirtyConfig, partial)
        },

        // 同步配置到后端
        async syncConfig() {
            if (Object.keys(this.dirtyConfig).length === 0) return

            try {
                await invoke("command_save_local_config", { partialConfig: this.dirtyConfig })
                this.dirtyConfig = {}
                await this.loadConfig()
            } catch (error) {
                console.error("同步配置失败:", error)
                throw error // 抛出错误以便 UI 处理
            }
        }
    },
})
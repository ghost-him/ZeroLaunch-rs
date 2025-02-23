import { defineStore } from 'pinia'
import { AppConfig, UIConfig, ProgramManagerConfig, ProgramLauncherConfig, ProgramLoaderConfig, PartialConfig } from '../api/types'
import { invoke } from '@tauri-apps/api/core'
import { merge } from 'lodash-es'

export const useConfigStore = defineStore('config', {
    state: () => ({
        app_config: {
            search_bar_placeholder: '',
            search_bar_no_result: '',
            is_auto_start: false,
            is_silent_start: false,
            search_result_count: 0,
            auto_refresh_time: 0
        } as AppConfig,
        ui_config: {
            item_width_scale_factor: 1,
            item_height_scale_factor: 1,
            selected_item_color: '',
            item_font_color: ''
        } as UIConfig,
        program_manager_config: {
            launcher: {
                launch_info: {},
                history_launch_time: {},
                last_update_date: ''
            } as ProgramLauncherConfig,
            loader: {
                target_paths: [],
                forbidden_paths: [],
                forbidden_program_key: [],
                program_bias: {},
                is_scan_uwp_programs: false,
                index_file_paths: [],
                index_web_pages: []
            } as ProgramLoaderConfig
        } as ProgramManagerConfig,
        dirtyConfig: {} as PartialConfig,
        }),
    actions: {
        // 从后端加载完整配置
        async loadConfig() {
            console.log("load from backend")
            const config = await invoke<PartialConfig>('load_config')
            console.log(typeof config.program_manager_config?.loader?.program_bias)
            this.$patch(config);

            console.log(typeof this.program_manager_config.loader.program_bias)
        },
        // 更新配置并同步到后端
        async updateConfig(partial: PartialConfig) {
            // 1. 更新本地状态
            this.$patch(partial);

            // 2. 深合并到 dirtyConfig
            this.dirtyConfig = merge({}, this.dirtyConfig, partial);
        },

        async syncConfig() {
            if (Object.keys(this.dirtyConfig).length === 0) return;

            try {
                console.log("向后端传输信息")
                console.log(this.dirtyConfig)
                await invoke("save_config", { partialConfig: this.dirtyConfig });
                this.dirtyConfig = {};
            } catch (error) {
                console.error("同步失败:", error);
            }
        },
    },
    getters: {
    }
})
import { defineStore } from 'pinia'
import { AppConfig, UIConfig, ProgramManagerConfig, ProgramLauncherConfig, ProgramLoaderConfig, PartialConfig, Config } from '../api/types'
import { invoke } from '@tauri-apps/api/core'
import { merge, cloneDeep, mergeWith } from 'lodash-es'

function mergeConfig(config: Config, partial: PartialConfig): Config {
    // 合并 app_config
    const app_config = partial.app_config
        ? { ...config.app_config, ...partial.app_config }
        : config.app_config;

    // 合并 ui_config
    const ui_config = partial.ui_config
        ? { ...config.ui_config, ...partial.ui_config }
        : config.ui_config;

    // 处理 program_manager_config
    const pmPartial = partial.program_manager_config;
    const pmConfig = config.program_manager_config;
    const program_manager_config = pmPartial
        ? {
              launcher: pmPartial.launcher
                  ? { ...pmConfig.launcher, ...pmPartial.launcher }
                  : pmConfig.launcher,
              loader: pmPartial.loader
                  ? { ...pmConfig.loader, ...pmPartial.loader }
                  : pmConfig.loader,
          }
        : pmConfig;

    // 返回合并后的新 Config 对象
    return {
        ...config,
        app_config,
        ui_config,
        program_manager_config,
    };
}

export const useConfigStore = defineStore('config', {
    state: () => ({
        config: {
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
            } as ProgramManagerConfig
        } as Config,
        dirtyConfig: {} as PartialConfig
    }),
    actions: {
        // 从后端加载完整配置
        async loadConfig() {
            console.log("load from backend")
            const config = await invoke<PartialConfig>('load_config')
            console.log(typeof config.program_manager_config?.loader?.program_bias)
            this.config = mergeConfig(this.config, config);
        },
        // 更新配置并同步到后端
        updateConfig(partial: PartialConfig) {
            // 1. 更新本地状态（带自定义合并规则）
            this.config = mergeConfig(this.config, partial);
            console.log(this.config.program_manager_config);
            // 2. 更新 dirtyConfig（带相同合并规则）
            this.dirtyConfig = merge({}, this.dirtyConfig, partial)
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
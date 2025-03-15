import { defineStore } from 'pinia'
import { AppConfig, UIConfig, ProgramManagerConfig, ProgramLauncherConfig, ProgramLoaderConfig, PartialConfig, Config, BlurStyle } from '../api/remote_config_types'
import { invoke } from '@tauri-apps/api/core'

function mergeConfig(config: Config , partial: PartialConfig): Config {
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
        ?{
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

function mergePartialConfig(
    partial1: PartialConfig,
    partial2: PartialConfig
): PartialConfig {
    // 合并 app_config
    const mergedAppConfig =
        partial1.app_config || partial2.app_config
            ? {
                  ...(partial1.app_config || {}),
                  ...(partial2.app_config || {}),
              }
            : undefined;

    // 合并 ui_config
    const mergedUiConfig =
        partial1.ui_config || partial2.ui_config
            ? {
                  ...(partial1.ui_config || {}),
                  ...(partial2.ui_config || {}),
              }
            : undefined;

    // 合并 program_manager_config
    const mergedProgramManagerConfig = mergePartialProgramManagerConfig(
        partial1.program_manager_config,
        partial2.program_manager_config
    );

    // 构建最终的 PartialConfig 对象
    const result: PartialConfig = {};
    if (mergedAppConfig !== undefined) result.app_config = mergedAppConfig;
    if (mergedUiConfig !== undefined) result.ui_config = mergedUiConfig;
    if (mergedProgramManagerConfig !== undefined)
        result.program_manager_config = mergedProgramManagerConfig;

    return result;
}

// 合并 program_manager_config 的辅助函数
function mergePartialProgramManagerConfig(
    pm1?: PartialConfig["program_manager_config"],
    pm2?: PartialConfig["program_manager_config"]
): PartialConfig["program_manager_config"] | undefined {
    if (!pm1 && !pm2) return undefined;

    // 合并 launcher
    const mergedLauncher =
        pm1?.launcher || pm2?.launcher
            ? {
                  ...(pm1?.launcher || {}),
                  ...(pm2?.launcher || {}),
              }
            : undefined;

    // 合并 loader
    const mergedLoader =
        pm1?.loader || pm2?.loader
            ? {
                  ...(pm1?.loader || {}),
                  ...(pm2?.loader || {}),
              }
            : undefined;

    // 构建最终的 program_manager_config 对象
    const mergedPm: PartialConfig["program_manager_config"] = {};
    if (mergedLauncher !== undefined) mergedPm.launcher = mergedLauncher;
    if (mergedLoader !== undefined) mergedPm.loader = mergedLoader;

    return Object.keys(mergedPm).length > 0 ? mergedPm : undefined;
}

export const useRemoteConfigStore = defineStore('config', {
    state: () => ({
        config: {
            app_config: {
                search_bar_placeholder: '',
                tips: '',
                is_auto_start: false,
                is_silent_start: false,
                search_result_count: 4,
                auto_refresh_time: 30,
                launch_new_on_failure: false,
                is_debug_mode: false,
            } as AppConfig,
            ui_config: {
                selected_item_color: '',
                item_font_color: '',
                search_bar_font_color: '',
                search_bar_font_size: 2.0,
                search_bar_background_color: '#FFFFFF00',
                item_font_size: 1.3,
                vertical_position_ratio: 0.4,
                search_bar_height: 65,
                result_item_height: 62,
                footer_height: 42,
                window_width: 1000,
                background_size: 'cover',
                background_position: 'center',
                background_repeat: 'no-repeat',
                background_opacity: 1,
                blur_style: BlurStyle.None,
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
                    index_web_pages: [],
                    custom_command: [],
                } as ProgramLoaderConfig
            } as ProgramManagerConfig
        } as Config,
        dirtyConfig: {} as PartialConfig
    }),
    actions: {
        // 从后端加载完整配置
        async loadConfig() {
            console.log("load from backend")
            const config = await invoke<PartialConfig>('command_load_remote_config')
            console.log(typeof config.program_manager_config?.loader?.program_bias)
            this.config = mergeConfig(this.config, config);
        },
        // 更新配置并同步到后端
        updateConfig(partial: PartialConfig) {
            console.log('更新消息');
            // 1. 更新本地状态（带自定义合并规则）
            this.config = mergeConfig(this.config, partial);
            // 2. 更新 dirtyConfig（带相同合并规则）
            this.dirtyConfig = mergePartialConfig(this.dirtyConfig, partial)
        },

        async syncConfig() {
            if (Object.keys(this.dirtyConfig).length === 0) return;

            try {
                await invoke("command_save_remote_config", { partialConfig: this.dirtyConfig });
                this.dirtyConfig = {};
            } catch (error) {
                console.error("同步失败:", error);
            }
        },
    },
    getters: {
    }
})
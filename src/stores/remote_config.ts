import { defineStore } from 'pinia'
import { default_ui_config,default_app_config, ProgramManagerConfig, ProgramLauncherConfig, ProgramLoaderConfig, PartialRemoteConfig, RemoteConfig, ImageLoaderConfig } from '../api/remote_config_types'
import { invoke } from '@tauri-apps/api/core'

function mergeConfig(config: RemoteConfig , partial: PartialRemoteConfig): RemoteConfig {
    // 合并 app_config
    const app_config = partial.app_config
        ? { ...config.app_config, ...partial.app_config }
        : config.app_config;

    // 合并 ui_config
    const ui_config = partial.ui_config
        ? { ...config.ui_config, ...partial.ui_config }
        : config.ui_config;

    // 合并 shortcut_config
    const shortcut_config = partial.shortcut_config ? { ...config.shortcut_config, ...partial.shortcut_config } : config.shortcut_config;

    // 处理 program_manager_config
    const pmPartial = partial.program_manager_config;
    const pmConfig = config.program_manager_config;
    const program_manager_config = pmPartial
        ?{
                launcher: pmPartial.launcher ? { ...pmConfig.launcher, ...pmPartial.launcher } : pmConfig.launcher,
                loader: pmPartial.loader ? { ...pmConfig.loader, ...pmPartial.loader } : pmConfig.loader,
                image_loader: pmPartial.image_loader ? { ...pmConfig.image_loader, ...pmPartial.image_loader}:pmConfig.image_loader,
                search_model: pmPartial.search_model ? pmPartial.search_model : pmConfig.search_model,
        }
        : pmConfig;
    // 返回合并后的新 Config 对象
    return {
        ...config,
        app_config,
        ui_config,
        shortcut_config,
        program_manager_config,
    };
}

function mergePartialConfig(
    partial1: PartialRemoteConfig,
    partial2: PartialRemoteConfig
): PartialRemoteConfig {
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

    // 合并shortcut
    const shortcutConfig = partial1.shortcut_config || partial2.shortcut_config ? {
        ...(partial1.shortcut_config || {}),
        ...(partial2.shortcut_config || {}),
    } : undefined;
    
    // 合并 program_manager_config
    const mergedProgramManagerConfig = mergePartialProgramManagerConfig(
        partial1.program_manager_config,
        partial2.program_manager_config
    );

    // 构建最终的 PartialConfig 对象
    const result: PartialRemoteConfig = {};
    if (mergedAppConfig !== undefined) result.app_config = mergedAppConfig;
    if (mergedUiConfig !== undefined) result.ui_config = mergedUiConfig;
    if (shortcutConfig !== undefined) result.shortcut_config = shortcutConfig;
    if (mergedProgramManagerConfig !== undefined)
        result.program_manager_config = mergedProgramManagerConfig;

    return result;
}

// 合并 program_manager_config 的辅助函数
function mergePartialProgramManagerConfig(
    pm1?: PartialRemoteConfig["program_manager_config"],
    pm2?: PartialRemoteConfig["program_manager_config"]
): PartialRemoteConfig["program_manager_config"] | undefined {
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
    // 合并 image_loader_config
    const mergedImageLoaderConfig =
        pm1?.image_loader || pm2?.image_loader
            ? {
                ...(pm1?.image_loader || {}),
                ...(pm2?.image_loader || {}),
            }
            : undefined;
    // 合并 search_model
    const mergedSearchModel = pm2?.search_model ?? pm1?.search_model;

    // 构建最终的 program_manager_config 对象
    const mergedPm: PartialRemoteConfig["program_manager_config"] = {};
    if (mergedLauncher !== undefined) mergedPm.launcher = mergedLauncher;
    if (mergedLoader !== undefined) mergedPm.loader = mergedLoader;
    if (mergedImageLoaderConfig !== undefined) mergedPm.image_loader = mergedImageLoaderConfig;
    if (mergedSearchModel !== undefined) mergedPm.search_model = mergedSearchModel;
    return Object.keys(mergedPm).length > 0 ? mergedPm : undefined;
}

export const useRemoteConfigStore = defineStore('config', {
    state: () => ({
        config: {
            app_config: default_app_config(),
            ui_config: default_ui_config(),
            program_manager_config: {
                launcher: {
                    launch_info: {},
                    history_launch_time: {},
                    last_update_date: ''
                } as ProgramLauncherConfig,
                loader: {
                    target_paths: [],
                    program_bias: {},
                    is_scan_uwp_programs: false,
                    index_web_pages: [],
                    custom_command: [],
                    forbidden_paths: [],
                    program_alias: {},
                } as ProgramLoaderConfig,
                image_loader: {
                    enable_icon_cache: true,
                    enable_online: true,
                } as ImageLoaderConfig,
            } as ProgramManagerConfig
        } as RemoteConfig,
        dirtyConfig: {} as PartialRemoteConfig
    }),
    actions: {
        // 从后端加载完整配置
        async loadConfig() {
            console.log("load from backend")
            const config = await invoke<PartialRemoteConfig>('command_load_remote_config')
            console.log(typeof config.program_manager_config?.loader?.program_bias)
            this.config = mergeConfig(this.config, config);
        },
        // 更新配置并同步到后端
        updateConfig(partial: PartialRemoteConfig) {
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
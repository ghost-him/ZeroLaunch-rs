import { defineStore } from 'pinia'
import { default_ui_config, default_app_config, default_shortcut_config, ProgramManagerConfig, ProgramLoaderConfig, PartialRemoteConfig, RemoteConfig, ImageLoaderConfig, ProgramRankerConfig } from '../api/remote_config_types'
import { invoke } from '@tauri-apps/api/core'

function mergeConfig(config: RemoteConfig, partial: PartialRemoteConfig): RemoteConfig {
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
        ? {
            ranker: pmPartial.ranker ? { ...pmConfig.ranker, ...pmPartial.ranker } : pmConfig.ranker,
            loader: pmPartial.loader ? { ...pmConfig.loader, ...pmPartial.loader } : pmConfig.loader,
            image_loader: pmPartial.image_loader ? { ...pmConfig.image_loader, ...pmPartial.image_loader } : pmConfig.image_loader,
            search_model: pmPartial.search_model ? pmPartial.search_model : pmConfig.search_model,
            enable_lru_search_cache: pmPartial.enable_lru_search_cache !== undefined ? pmPartial.enable_lru_search_cache : pmConfig.enable_lru_search_cache,
            search_cache_capacity: pmPartial.search_cache_capacity !== undefined ? pmPartial.search_cache_capacity : pmConfig.search_cache_capacity,
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

    // 合并 ranker
    const mergedRanker =
        pm1?.ranker || pm2?.ranker
            ? {
                ...(pm1?.ranker || {}),
                ...(pm2?.ranker || {}),
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
    const mergedEnableCache = pm2?.enable_lru_search_cache ?? pm1?.enable_lru_search_cache;
    const mergedCacheCapacity = pm2?.search_cache_capacity ?? pm1?.search_cache_capacity;

    // 构建最终的 program_manager_config 对象
    const mergedPm: PartialRemoteConfig["program_manager_config"] = {};
    if (mergedRanker !== undefined) mergedPm.ranker = mergedRanker;
    if (mergedLoader !== undefined) mergedPm.loader = mergedLoader;
    if (mergedImageLoaderConfig !== undefined) mergedPm.image_loader = mergedImageLoaderConfig;
    if (mergedSearchModel !== undefined) mergedPm.search_model = mergedSearchModel;
    if (mergedEnableCache !== undefined) mergedPm.enable_lru_search_cache = mergedEnableCache;
    if (mergedCacheCapacity !== undefined) mergedPm.search_cache_capacity = mergedCacheCapacity;
    return Object.keys(mergedPm).length > 0 ? mergedPm : undefined;
}

export const useRemoteConfigStore = defineStore('config', {
    state: () => ({
        config: {
            app_config: default_app_config(),
            ui_config: default_ui_config(),
            shortcut_config: default_shortcut_config(),
            program_manager_config: {
                ranker: {
                    history_weight: 1.2,
                    recent_habit_weight: 2.5,
                    temporal_weight: 0.8,
                    query_affinity_weight: 3.5,
                    query_affinity_time_decay: 259200,
                    temporal_decay: 10800,
                    is_enable: true
                } as ProgramRankerConfig,
                loader: {
                    target_paths: [],
                    program_bias: {},
                    is_scan_uwp_programs: false,
                    index_web_pages: [],
                    custom_command: [],
                    forbidden_paths: [],
                    program_alias: {},
                    enabled_builtin_commands: {
                        OpenSettings: true,
                        RefreshDatabase: true,
                        RetryRegisterShortcut: true,
                        ToggleGameMode: true,
                        ExitProgram: true,
                    },
                    builtin_command_keywords: {
                        OpenSettings: [],
                        RefreshDatabase: [],
                        RetryRegisterShortcut: [],
                        ToggleGameMode: [],
                        ExitProgram: [],
                    }
                } as ProgramLoaderConfig,
                image_loader: {
                    enable_icon_cache: true,
                    enable_online: true,
                } as ImageLoaderConfig,
                search_model: 'standard',
                enable_lru_search_cache: false,
                search_cache_capacity: 120,
            } as ProgramManagerConfig
        } as RemoteConfig,
        dirtyConfig: {} as PartialRemoteConfig
    }),
    actions: {
        // 从后端加载完整配置
        async loadConfig() {

            try {
                const config = await invoke<PartialRemoteConfig>('command_load_remote_config')
                this.config = mergeConfig(this.config, config);
            } catch (e) {
                console.error("Failed to load config", e);
            }
        },
        // 更新配置并同步到后端
        updateConfig(partial: PartialRemoteConfig) {

            // 1. 更新本地状态（带自定义合并规则）
            this.config = mergeConfig(this.config, partial);
            // 2. 更新 dirtyConfig（带相同合并规则）
            this.dirtyConfig = mergePartialConfig(this.dirtyConfig, partial)
        },
        async syncConfig() {
            if (Object.keys(this.dirtyConfig).length === 0) return;
            try {
                await invoke('command_save_remote_config', { partialConfig: this.dirtyConfig });
                this.dirtyConfig = {};
            } catch (e) {
                console.error("Failed to sync config", e);
            }
        }
    }
});

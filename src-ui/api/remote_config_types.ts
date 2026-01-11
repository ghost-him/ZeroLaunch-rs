
export type Shortcut = {
    key: string,
    ctrl: boolean,
    alt: boolean,
    shift: boolean,
    meta: boolean,
}

export type ShortcutConfig = {
    open_search_bar: Shortcut,
    switch_to_everything: Shortcut,
    arrow_up: Shortcut,
    arrow_down: Shortcut,
    arrow_left: Shortcut,
    arrow_right: Shortcut,
    double_click_ctrl: boolean,
}

export function default_shortcut_config(): ShortcutConfig {
    return {
        open_search_bar: {
            key: 'Space',
            ctrl: false,
            alt: true,
            shift: false,
            meta: false,
        },
        switch_to_everything: {
            key: 'e',
            ctrl: true,
            alt: false,
            shift: false,
            meta: false,
        },
        arrow_up: {
            key: 'k',
            ctrl: true,
            alt: false,
            shift: false,
            meta: false,
        },
        arrow_down: {
            key: 'j',
            ctrl: true,
            alt: false,
            shift: false,
            meta: false,
        },
        arrow_left: {
            key: 'h',
            ctrl: true,
            alt: false,
            shift: false,
            meta: false,
        },
        arrow_right: {
            key: 'l',
            ctrl: true,
            alt: false,
            shift: false,
            meta: false,
        },
        double_click_ctrl: false,
    } as ShortcutConfig
}

export type LogLevel = 'debug' | 'info' | 'warn' | 'error';

export type AppConfig = {
    search_bar_placeholder: string
    tips: string
    is_auto_start: boolean
    is_silent_start: boolean
    search_result_count: number
    launch_new_on_failure: boolean
    is_debug_mode: boolean
    is_esc_hide_window_priority: boolean,
    is_enable_drag_window: boolean,
    window_position: [number, number],
    is_wake_on_fullscreen: boolean,
    space_is_enter: boolean,
    show_pos_follow_mouse: boolean,
    scroll_threshold: number,
    language: string,
    log_level: LogLevel,
}

export function default_app_config(): AppConfig {
    return {
        search_bar_placeholder: '',
        tips: '',
        is_auto_start: false,
        is_silent_start: false,
        search_result_count: 4,
        launch_new_on_failure: false,
        is_debug_mode: false,
        is_esc_hide_window_priority: false,
        is_enable_drag_window: false,
        window_position: [0, 0],
        is_wake_on_fullscreen: false,
        space_is_enter: false,
        show_pos_follow_mouse: false,
        scroll_threshold: 10,
        language: 'zh-Hans',
        log_level: 'info',
    } as AppConfig
}

export type ThemeMode = 'system' | 'light' | 'dark';

export type UIConfig = {
    frontend_theme_mode: ThemeMode,
    tray_theme_mode: ThemeMode,
    selected_item_color: string
    item_font_color: string
    search_bar_font_color: string
    search_bar_background_color: string
    item_font_size: number
    search_bar_font_size: number
    vertical_position_ratio: number
    search_bar_height: number,
    result_item_height: number,
    footer_height: number,
    window_width: number,
    background_size: string,
    background_position: string,
    background_repeat: string,
    background_opacity: number,
    blur_style: string,
    search_bar_placeholder_font_color: string,
    window_corner_radius: number,
    use_windows_sys_control_radius: boolean,
    footer_font_size: number,
    footer_font_color: string,
    search_bar_font_family: string,
    result_item_font_family: string,
    footer_font_family: string,
    program_background_color: string,
    search_bar_animate: boolean,
    show_launch_command: boolean,
}

export function default_ui_config(): UIConfig {
    return {
        frontend_theme_mode: 'system',
        tray_theme_mode: 'system',
        selected_item_color: '#e3e3e3cc',
        item_font_color: '#000000',
        search_bar_font_color: '#333333',
        search_bar_font_size: 50,
        search_bar_background_color: '#FFFFFF00',
        item_font_size: 33,
        vertical_position_ratio: 0.4,
        search_bar_height: 65,
        result_item_height: 62,
        footer_height: 42,
        window_width: 1000,
        background_size: 'cover',
        background_position: 'center',
        background_repeat: 'no-repeat',
        background_opacity: 1,
        blur_style: 'None',
        search_bar_placeholder_font_color: '#757575',
        window_corner_radius: 16,
        use_windows_sys_control_radius: false,
        footer_font_size: 33,
        footer_font_color: '#666666',
        search_bar_font_family: 'Segoe UI',
        result_item_font_family: 'Segoe UI',
        footer_font_family: 'Segoe UI',
        program_background_color: '#FFFFFFFF',
        search_bar_animate: true,
        show_launch_command: false,
    } as UIConfig
}

export type ProgramRankerConfig = {
    history_weight: number
    recent_habit_weight: number
    temporal_weight: number
    query_affinity_weight: number
    query_affinity_time_decay: number
    query_affinity_cooldown: number
    temporal_decay: number
    is_enable: boolean
}

export type SymlinkMode = 'ExplicitOnly' | 'Auto';

export type DirectoryConfig = {
    root_path: string
    max_depth: number
    pattern: string[]
    pattern_type: string
    excluded_keywords: string[]
    symlink_mode?: SymlinkMode
    max_symlink_depth?: number
}

export type BuiltinCommandType =
    | 'OpenSettings'
    | 'RefreshDatabase'
    | 'RetryRegisterShortcut'
    | 'ToggleGameMode'
    | 'ExitProgram';

export type ProgramLoaderConfig = {
    target_paths: DirectoryConfig[]
    program_bias: { [key: string]: [number, string] }
    is_scan_uwp_programs: boolean
    index_web_pages: [string, string][]
    custom_command: [string, string][]
    forbidden_paths: string[]
    program_alias: { [key: string]: string[] }
    enabled_builtin_commands: Record<BuiltinCommandType, boolean>
    builtin_command_keywords: Record<BuiltinCommandType, string[]>
}

export type IconManagerConfig = {
    enable_icon_cache: boolean,
    enable_online: boolean,
}

export type ProgramManagerConfig = {
    ranker: ProgramRankerConfig
    loader: ProgramLoaderConfig
    enable_lru_search_cache: boolean
    search_cache_capacity: number
    search_model: string
}

export type EverythingSortKind =
    | 'NameAscending'
    | 'NameDescending'
    | 'PathAscending'
    | 'PathDescending'
    | 'SizeAscending'
    | 'SizeDescending'
    | 'ExtensionAscending'
    | 'ExtensionDescending'
    | 'TypeNameAscending'
    | 'TypeNameDescending'
    | 'DateCreatedAscending'
    | 'DateCreatedDescending'
    | 'DateModifiedAscending'
    | 'DateModifiedDescending'
    | 'AttributesAscending'
    | 'AttributesDescending'
    | 'FileListFilenameAscending'
    | 'FileListFilenameDescending'
    | 'RunCountAscending'
    | 'RunCountDescending'
    | 'DateRecentlyChangedAscending'
    | 'DateRecentlyChangedDescending'
    | 'DateAccessedAscending'
    | 'DateAccessedDescending'
    | 'DateRunAscending'
    | 'DateRunDescending';

// Everything 页面特有的快捷键配置
export type EverythingShortcutConfig = {
    enable_path_match: Shortcut,  // 在资源管理器中打开选中项
}

export function default_everything_shortcut_config(): EverythingShortcutConfig {
    return {
        enable_path_match: {
            key: 'u',
            ctrl: true,
            alt: false,
            shift: false,
            meta: false,
        },
    }
}

export type EverythingConfig = {
    sort_threshold: number
    sort_method: EverythingSortKind
    result_limit: number
    shortcuts: EverythingShortcutConfig
}

export function default_everything_config(): EverythingConfig {
    return {
        sort_threshold: 3,
        sort_method: 'NameAscending',
        result_limit: 10,
        shortcuts: default_everything_shortcut_config(),
    } as EverythingConfig
}

// 刷新调度器配置
export type RefreshSchedulerConfig = {
    auto_refresh_interval_mins: number
    enable_installation_monitor: boolean
    monitor_debounce_secs: number
}

export function default_refresh_scheduler_config(): RefreshSchedulerConfig {
    return {
        auto_refresh_interval_mins: 30,
        enable_installation_monitor: false,
        monitor_debounce_secs: 5,
    } as RefreshSchedulerConfig
}

// 书签源配置
export type BookmarkSourceConfig = {
    name: string
    bookmarks_path: string
    enabled: boolean
}

// 书签覆盖配置
export type BookmarkOverride = {
    url: string
    excluded: boolean
    custom_title: string | null
}

export type BookmarkLoaderConfig = {
    sources: BookmarkSourceConfig[]
    overrides: BookmarkOverride[]
}

export function default_bookmark_loader_config(): BookmarkLoaderConfig {
    return {
        sources: [],
        overrides: [],
    }
}

export type RemoteConfig = {
    app_config: AppConfig
    ui_config: UIConfig
    shortcut_config: ShortcutConfig
    program_manager_config: {
        ranker: ProgramRankerConfig
        loader: ProgramLoaderConfig
        search_model: string
        enable_lru_search_cache: boolean
        search_cache_capacity: number
    }
    icon_manager_config: IconManagerConfig
    everything_config: EverythingConfig
    refresh_scheduler_config: RefreshSchedulerConfig
    bookmark_loader_config: BookmarkLoaderConfig
}

export type PartialAppConfig = Partial<AppConfig>
export type PartialUIConfig = Partial<UIConfig>
export type PartialProgramRankerConfig = Partial<ProgramRankerConfig>
export type PartialProgramLoaderConfig = Partial<ProgramLoaderConfig>
export type PartialIconManagerConfig = Partial<IconManagerConfig>
export type PartialShortcutConfig = Partial<ShortcutConfig>
export type PartialEverythingShortcutConfig = Partial<EverythingShortcutConfig>
export type PartialEverythingConfig = Partial<Omit<EverythingConfig, 'shortcuts'>> & {
    shortcuts?: PartialEverythingShortcutConfig
}
export type PartialRefreshSchedulerConfig = Partial<RefreshSchedulerConfig>
export type PartialBookmarkLoaderConfig = Partial<BookmarkLoaderConfig>

export type PartialRemoteConfig = {
    app_config?: PartialAppConfig
    ui_config?: PartialUIConfig
    shortcut_config?: PartialShortcutConfig
    program_manager_config?: {
        ranker?: PartialProgramRankerConfig
        loader?: PartialProgramLoaderConfig
        search_model?: Partial<string>
        enable_lru_search_cache?: boolean
        search_cache_capacity?: number
    }
    icon_manager_config?: PartialIconManagerConfig
    everything_config?: PartialEverythingConfig
    refresh_scheduler_config?: PartialRefreshSchedulerConfig
    bookmark_loader_config?: PartialBookmarkLoaderConfig
}


export type Shortcut = {
    key: string,
    ctrl: boolean,
    alt: boolean,
    shift: boolean,
    meta: boolean,
}

export type ShortcutConfig = {
    open_search_bar: Shortcut,
    arrow_up: Shortcut,
    arrow_down: Shortcut,
    arrow_left: Shortcut,
    arrow_right: Shortcut,
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
    } as ShortcutConfig
}

export type LogLevel = 'debug' | 'info' | 'warn' | 'error';

export type AppConfig = {
    search_bar_placeholder: string
    tips: string
    is_auto_start: boolean
    is_silent_start: boolean
    search_result_count: number
    auto_refresh_time: number
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
        auto_refresh_time: 30,
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

export type UIConfig = {
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
}

export function default_ui_config(): UIConfig {
    return {
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
    } as UIConfig
}

export type ProgramRankerConfig = {
    history_weight: number
    recent_habit_weight: number
    temporal_weight: number
    query_affinity_weight: number
    query_affinity_time_decay: number
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

export type ImageLoaderConfig = {
    enable_icon_cache: boolean,
    enable_online: boolean,
}

export type ProgramManagerConfig = {
    ranker: ProgramRankerConfig
    loader: ProgramLoaderConfig
    image_loader: ImageLoaderConfig
    enable_lru_search_cache: boolean
    search_cache_capacity: number
    search_model: string
}

export type RemoteConfig = {
    app_config: AppConfig
    ui_config: UIConfig
    shortcut_config: ShortcutConfig
    program_manager_config: {
        ranker: ProgramRankerConfig
        loader: ProgramLoaderConfig
        image_loader: ImageLoaderConfig
        search_model: string
        enable_lru_search_cache: boolean
        search_cache_capacity: number
    }
}

export type PartialAppConfig = Partial<AppConfig>
export type PartialUIConfig = Partial<UIConfig>
export type PartialProgramRankerConfig = Partial<ProgramRankerConfig>
export type PartialProgramLoaderConfig = Partial<ProgramLoaderConfig>
export type PartialImageLoaderConfig = Partial<ImageLoaderConfig>
export type PartialShortcutConfig = Partial<ShortcutConfig>

export type PartialRemoteConfig = {
    app_config?: PartialAppConfig
    ui_config?: PartialUIConfig
    shortcut_config?: PartialShortcutConfig
    program_manager_config?: {
        ranker?: PartialProgramRankerConfig
        loader?: PartialProgramLoaderConfig
        image_loader?: PartialImageLoaderConfig
        search_model?: Partial<string>
        enable_lru_search_cache?: boolean
        search_cache_capacity?: number
    }
}

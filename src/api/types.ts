export type AppConfig = {
    search_bar_placeholder: string
    search_bar_no_result: string
    is_auto_start: boolean
    is_silent_start: boolean
    search_result_count: number
    auto_refresh_time: number
}

export type UIConfig = {
    item_width_scale_factor: number
    item_height_scale_factor: number
    selected_item_color: string
    item_font_color: string
}

export type ProgramLauncherConfig = {
    launch_info: { [key: string]: number }
    history_launch_time: { [key: string]: number }
    last_update_date: string
}

export type ProgramLoaderConfig = {
    target_paths: string[]
    forbidden_paths: string[]
    forbidden_program_key: string[]
    program_bias: { [key: string]: [number, string] }
    is_scan_uwp_programs: boolean
    index_file_paths: string[]
    index_web_pages: [string, string][]
}

export type ProgramManagerConfig = {
    launcher: ProgramLauncherConfig
    loader: ProgramLoaderConfig
}

export type PartialAppConfig = Partial<AppConfig>
export type PartialUIConfig = Partial<UIConfig>
export type PartialProgramLauncherConfig = Partial<ProgramLauncherConfig>
export type PartialProgramLoaderConfig = Partial<ProgramLoaderConfig>

export type PartialConfig = {
    app_config?: PartialAppConfig
    ui_config?: PartialUIConfig
    program_manager_config?: {
        launcher?: PartialProgramLauncherConfig
        loader?: PartialProgramLoaderConfig
    }
}
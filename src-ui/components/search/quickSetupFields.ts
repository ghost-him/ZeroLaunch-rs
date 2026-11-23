/** 快速设置页要展示的字段引用：只声明组件 id + 字段 key。
 *  控件、可选项与校验均由宿主 schema 提供，页面不感知字段种类 —— 新增项只需在此加一行。 */
export interface QuickSetupFieldRef {
  componentId: string
  key: string
}

export const QUICK_SETUP_FIELDS: QuickSetupFieldRef[] = [
  { componentId: 'general-config', key: 'language' },
  { componentId: 'general-config', key: 'is_auto_start' },
  { componentId: 'general-config', key: 'show_update_notes' },
  { componentId: 'window-behavior-config', key: 'is_show_home_on_empty_query' },
  { componentId: 'window-behavior-config', key: 'auto_game_mode_on_fullscreen' },
]

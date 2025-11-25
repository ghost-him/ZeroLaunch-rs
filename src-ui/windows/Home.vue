<template>
  <div
    class="launcher-container"
    tabindex="0"
    :style="[program_backgroundStyle,
             !effective_ui_config.use_windows_sys_control_radius ? {
               border: `1px solid ${is_dark ? '#3d3d3d' : '#bdbdbd'}`,
               borderRadius: `${effective_ui_config.window_corner_radius}px`
             } : {}]"
    @keydown="handleKeyDown"
    @keyup="handleKeyUp"
    @blur="handleBlur"
  >
    <div class="unified-container">
      <!-- Search Bar -->
      <SearchBar
        ref="searchBarRef"
        v-model="searchText"
        :app-config="app_config"
        :ui-config="effective_ui_config"
        @contextmenu="contextSearchBarEvent"
      />

      <!-- Parameter Panel -->
      <ParameterPanel
        v-if="parameterSession"
        ref="parameterPanelRef"
        v-model:input-value="parameterSession.inputValue"
        :ui-config="effective_ui_config"
        :prompt="parameterPrompt"
        :progress="`${parameterSession.collectedArgs.length + 1}/${parameterSession.info.placeholderCount}`"
        :action-label="parameterActionLabel"
        :preview="parameterPreview"
        @confirm="confirmParameterInput"
        @cancel="cancelParameterSession"
      />

      <!-- Result List -->
      <ResultList
        v-if="!isEverythingMode"
        ref="resultsListRef"
        :menu-items="menuItems"
        :menu-icons="menuIcons"
        :selected-index="selectedIndex"
        :ui-config="effective_ui_config"
        :app-config="app_config"
        :hover-color="hover_item_color"
        :is-scroll-mode="isScrollMode"
        @item-click="handleItemClick"
        @item-contextmenu="contextResultItemEvent"
      />

      <!-- Everything Panel -->
      <EverythingPanel
        v-else
        ref="everythingPanelRef"
        :search-text="searchText"
        :ui-config="effective_ui_config"
        :app-config="app_config"
        :hover-color="hover_item_color"
      />
    </div>

    <!-- Footer -->
    <Footer
      :ui-config="effective_ui_config"
      :app-config="app_config"
      :status-text="is_refreshing_dataset ? t('app.refreshing_dataset') : (is_loading_icons ? t('app.loading_icons') : (status_tip || right_tips))"
    />

    <!-- SubMenus -->
    <SubMenu
      ref="searchBarMenuBuf"
      :item-height="effective_ui_config.result_item_height"
      :window-size="innerWindowSize"
      :menu-items="searchBarMenuItems"
      :is-dark="is_dark"
      :corner-radius="effective_ui_config.window_corner_radius"
      :hover-color="hover_item_color"
      :selected-color="effective_ui_config.selected_item_color"
      :item-font-color="effective_ui_config.item_font_color"
      :item-font-size-percent="effective_ui_config.item_font_size"
      :style="submenu_backgroundStyle"
    />

    <SubMenu
      ref="resultItemMenuRef"
      :item-height="effective_ui_config.result_item_height"
      :window-size="innerWindowSize"
      :menu-items="resultSubMenuItems"
      :is-dark="is_dark"
      :corner-radius="effective_ui_config.window_corner_radius"
      :hover-color="hover_item_color"
      :selected-color="effective_ui_config.selected_item_color"
      :item-font-color="effective_ui_config.item_font_color"
      :item-font-size-percent="effective_ui_config.item_font_size"
      :style="submenu_backgroundStyle"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch, onUnmounted, nextTick } from 'vue'
import { listen, UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useI18n } from 'vue-i18n'
import { FolderOpened, Refresh, Setting, StarFilled } from '@element-plus/icons-vue'

import { reduceOpacity } from '../utils/color'
import { initializeLanguage } from '../i18n'
import { 
  AppConfig, default_app_config, default_ui_config, PartialAppConfig, 
  PartialUIConfig, ShortcutConfig, UIConfig, default_shortcut_config, 
  PartialShortcutConfig, 
} from '../api/remote_config_types'

import { useShortcuts } from '../composables/useShortcuts'
import SearchBar from './search-window-components/SearchBar.vue'
import ResultList from './search-window-components/ResultList.vue'
import EverythingPanel from './search-window-components/EverythingPanel.vue'
import Footer from './search-window-components/Footer.vue'
import ParameterPanel from './search-window-components/ParameterPanel.vue'
import SubMenu from './search-window-components/SubMenu.vue'

const { t } = useI18n()

// State
const app_config = ref<AppConfig>(default_app_config())
const ui_config = ref<UIConfig>(default_ui_config())
const shortcut_config = ref<ShortcutConfig>(default_shortcut_config())
const searchText = ref('')
const selectedIndex = ref<number>(0)
const searchBarRef = ref<InstanceType<typeof SearchBar> | null>(null)
const resultsListRef = ref<InstanceType<typeof ResultList> | null>(null)
const everythingPanelRef = ref<InstanceType<typeof EverythingPanel> | null>(null)
const searchResults = ref<Array<[number, string]>>([])
const latest_launch_program = ref<Array<[number, string]>>([])
const is_alt_pressed = ref<boolean>(false)
const right_tips = ref<string>(t('app.best_match'))
const status_reason_code = ref<string>('none')
const menuItems = ref<Array<string>>([])
const menuIcons = ref<Array<string>>([])
const program_icons = ref<Map<number, string>>(new Map<number, string>([]))
const is_visible = ref<boolean>(false)
const background_picture = ref('')
const is_loading_icons = ref<boolean>(false)
const is_refreshing_dataset = ref<boolean>(false)
const is_dark = ref(false)
const darkModeMediaQuery = ref<MediaQueryList | null>(null)
const windowSize = ref<{ width: number; height: number; }>({ width: 800, height: 800 })
const scaleFactor = ref<number>(1)
const isEverythingMode = ref(false)

const effective_ui_config = computed(() => {
  if (!is_dark.value) {
    return ui_config.value
  }
  return {
    ...ui_config.value,
    program_background_color: 'rgba(31, 31, 31, 1)',
    selected_item_color: 'rgba(63, 63, 63, 0.8)',
    item_font_color: '#A6A6A6',
    search_bar_font_color: '#A6A6A6',
    search_bar_placeholder_font_color: '#757575',
    footer_font_color: '#A6A6A6',
  }
})

const toggleEverythingMode = async () => {
  isEverythingMode.value = !isEverythingMode.value
  searchResults.value = []
  selectedIndex.value = 0
  // No need to call sendSearchText here, EverythingPanel will react to searchText prop
}

// Parameter Session Types
type LaunchMethodKind = 'Path' | 'PackageFamilyName' | 'File' | 'Command';

interface LaunchTemplateInfo {
  template: string;
  kind: LaunchMethodKind;
  placeholderCount: number;
  showName: string;
}

interface ParameterSession {
  programGuid: number;
  ctrlKey: boolean;
  shiftKey: boolean;
  info: LaunchTemplateInfo;
  collectedArgs: string[];
  inputValue: string;
}

interface LaunchTemplateInfoResponse {
  template: string;
  kind: LaunchMethodKind;
  placeholder_count: number;
  show_name: string;
}

const parameterSession = ref<ParameterSession | null>(null)
const parameterPanelRef = ref<InstanceType<typeof ParameterPanel> | null>(null)

// Computed
const status_tip = computed(() => {
  switch (status_reason_code.value) {
    case 'ai_disabled':
      return t('app.semantic_fallback_ai_disabled')
    case 'model_not_ready':
      return t('app.semantic_fallback_model_missing')
    default:
      return ''
  }
})

const hover_item_color = computed(() => {
  return reduceOpacity(effective_ui_config.value.selected_item_color, 0.8)
})

const isScrollMode = computed(() => {
  const currentResults = is_alt_pressed.value ? latest_launch_program.value : searchResults.value
  return currentResults.length > app_config.value.scroll_threshold
})

const parameterPrompt = computed(() => {
  if (!parameterSession.value) {
    return ''
  }
  const currentIndex = parameterSession.value.collectedArgs.length + 1
  return t('parameter.prompt', {
    index: currentIndex,
    total: parameterSession.value.info.placeholderCount,
    program: parameterSession.value.info.showName,
  })
})

const parameterActionLabel = computed(() => {
  if (!parameterSession.value) {
    return t('parameter.next')
  }
  const isLast =
    parameterSession.value.collectedArgs.length + 1 >=
    parameterSession.value.info.placeholderCount
  return isLast ? t('parameter.launch') : t('parameter.next')
})

const parameterPreview = computed(() => {
  if (!parameterSession.value) {
    return ''
  }
  const { info, collectedArgs, inputValue } = parameterSession.value
  const provisionalArgs = [...collectedArgs]
  if (provisionalArgs.length < info.placeholderCount) {
    provisionalArgs.push(inputValue)
  }
  while (provisionalArgs.length < info.placeholderCount) {
    provisionalArgs.push('…')
  }
  return buildTemplatePreview(info.template, provisionalArgs, info.placeholderCount)
})

const innerWindowSize = computed(() => {
  return {
    width: Math.round(windowSize.value.width / scaleFactor.value),
    height: Math.round(windowSize.value.height / scaleFactor.value),
  }
})

const submenu_backgroundStyle = computed(() => ({
  backgroundColor: `${effective_ui_config.value.program_background_color}`,
}))

const program_backgroundStyle = computed(() => ({
  backgroundColor: (effective_ui_config.value.blur_style !== 'None' && effective_ui_config.value.use_windows_sys_control_radius === true)
    ? 'transparent'
    : effective_ui_config.value.program_background_color,
  backgroundImage: `linear-gradient(rgba(255, 255, 255, ${1 - effective_ui_config.value.background_opacity}), rgba(255, 255, 255, ${1 - effective_ui_config.value.background_opacity})), url(${background_picture.value})`,
  backgroundSize: `${effective_ui_config.value.background_size}`,
  backgroundPosition: `${effective_ui_config.value.background_position}`,
  backgroundRepeat: `${effective_ui_config.value.background_repeat}`,
  backgroundClip: 'content-box',
}))

// SubMenus
const searchBarMenuBuf = ref<InstanceType<typeof SubMenu> | null>(null)
const resultItemMenuRef = ref<InstanceType<typeof SubMenu> | null>(null)

const searchBarMenuItems = computed(() => [{ name: t('menu.open_settings'), icon: Setting, action: () => { openSettingsWindow() } },
{ name: t('menu.refresh_database'), icon: Refresh, action: () => { refreshDataset() } }])

const resultSubMenuItems = computed(() => [{ name: t('app.open_file_location'), icon: FolderOpened, action: () => { openFolder() } },
{ name: t('app.run_as_admin'), icon: StarFilled, action: () => { runTargetProgramWithAdmin() } }])


// Methods
const buildTemplatePreview = (template: string, args: string[], placeholderCount: number) => {
  let result = ''
  let remaining = template
  let index = 0

  while (true) {
    const placeholderIndex = remaining.indexOf('{}')
    if (placeholderIndex === -1) {
      result += remaining
      break
    }

    result += remaining.slice(0, placeholderIndex)
    const replacement = index < args.length ? args[index] : '{}'
    result += replacement

    remaining = remaining.slice(placeholderIndex + 2)
    index += 1
  }

  if (args.length > placeholderCount) {
    const extraArgs = args.slice(placeholderCount).join(' ')
    if (extraArgs.length > 0) {
      result += ` ${extraArgs}`
    }
  }

  return result
}

const sendSearchText = async (text: string) => {
  try {
    if (isEverythingMode.value) {
      // Everything mode handles search internally in EverythingPanel via prop watch
      return
    }
    let results: Array<[number, string]>
    results = await invoke('handle_search_text', { searchText: text })
    searchResults.value = results
    await refresh_result_items()
    selectedIndex.value = 0
    if (resultsListRef.value?.resultsListRef) {
      resultsListRef.value.resultsListRef.scrollTop = 0
    }
  } catch (error) {
    console.error('Error sending search text to Rust: ', error)
  }
}

const resetParameterSession = () => {
  parameterSession.value = null
}

const startParameterSession = (programGuid: number, ctrlKey: boolean, shiftKey: boolean, info: LaunchTemplateInfoResponse) => {
  const sessionInfo: LaunchTemplateInfo = {
    template: info.template,
    kind: info.kind,
    placeholderCount: info.placeholder_count,
    showName: info.show_name,
  }

  parameterSession.value = {
    programGuid,
    ctrlKey,
    shiftKey,
    info: sessionInfo,
    collectedArgs: [],
    inputValue: '',
  }
}

const cancelParameterSession = () => {
  resetParameterSession()
}

// 提交当前输入并在占位符收集完成后发起启动
const confirmParameterInput = async () => {
  if (!parameterSession.value) {
    return
  }

  const session = parameterSession.value
  session.collectedArgs.push(session.inputValue)

  if (session.collectedArgs.length < session.info.placeholderCount) {
    session.inputValue = ''
    await nextTick()
    parameterPanelRef.value?.focus()
    return
  }

  try {
    await invoke('launch_program', {
      programGuid: session.programGuid,
      ctrl: session.ctrlKey,
      shift: session.shiftKey,
      args: session.collectedArgs,
    })
  } catch (error) {
    console.error('Failed to launch program with arguments:', error)
  } finally {
    resetParameterSession()
  }
}

const get_latest_launch_program = async () => {
  const results: Array<[number, string]> = await invoke('command_get_latest_launch_program')
  latest_launch_program.value = results
  await refresh_result_items()
}

const refresh_result_items = async () => {
  if (!is_alt_pressed.value) {
    menuItems.value = searchResults.value.map(([_id, item]: [number, string]) => item)
    const keys = searchResults.value.map(([key]: [number, string]) => key)
    if (isEverythingMode.value) {
      menuIcons.value = new Array(keys.length).fill('/tauri.svg')
      right_tips.value = 'Everything Search'
    } else {
      menuIcons.value = await getIcons(keys)
      right_tips.value = t('app.best_match')
    }
  } else {
    menuItems.value = latest_launch_program.value.map(([_id, item]: [number, string]) => item)
    const keys = latest_launch_program.value.map(([key]: [number, string]) => key)
    menuIcons.value = await getIcons(keys)
    right_tips.value = t('app.recent_open')
  }
  try {
    status_reason_code.value = await invoke<string>('command_get_search_status_tip')
  } catch (e) {
    // ignore
  }
}

const contextSearchBarEvent = (event: MouseEvent) => {
  if (resultItemMenuRef.value?.isVisible()) {
    resultItemMenuRef.value?.hideMenu()
  }
  searchBarMenuBuf.value?.showMenu({ top: event.clientY, left: event.clientX })
}

const openSettingsWindow = () => {
  invoke('show_setting_window')
    .then(() => {

    })
    .catch((error: any) => {
      console.error('Failed to open settings window:', error)
    })
}

const refreshDataset = async () => {

  await invoke('hide_window')
  await invoke('refresh_program')
  updateWindow()
}

const updateWindow = async () => {
  console.log('updateWindow')
  try {
    const [background_picture_data, program_count, data] = await Promise.all([
      invoke<number[]>('get_background_picture'),
      invoke<number>('get_program_count'),
      invoke<[PartialAppConfig, PartialUIConfig, PartialShortcutConfig]>('update_search_bar_window'),
    ])

    app_config.value = { ...app_config.value, ...data[0] }
    ui_config.value = { ...ui_config.value, ...data[1] }
    shortcut_config.value = { ...shortcut_config.value, ...data[2] }
    await initializeLanguage(app_config.value.language)

    const blob = new Blob([new Uint8Array(background_picture_data)], { type: 'image/png' })
    const url = URL.createObjectURL(blob)

    background_picture.value = url

    program_icons.value.forEach((url: string) => URL.revokeObjectURL(url))
    program_icons.value.clear()

    if (!is_visible.value || searchText.value.length == 0) {
      // 如果没有这个，那么就会导致在没有更新完成时，结果栏也是空的，这样不好看，所以提前发送一次搜索文本
      await sendSearchText('')
    }
    await startPreloadResource(program_count).then(async () => {
      is_loading_icons.value = false
      // 如果没有这个，那么可能会导致图标加载不正确（显示是空的），加了以后会再次搜索，从而显示正确的图标
      if (!is_visible.value || searchText.value.length == 0) {
        await sendSearchText('')
      }
    })

  } catch (error) {
    console.error('Error in updateWindow:', error)
  }
}

const startPreloadResource = async (program_count: number) => {
  is_loading_icons.value = true
  const BATCH_SIZE = 100

  program_icons.value.forEach((url: string) => URL.revokeObjectURL(url))
  program_icons.value.clear()

  const allIds = Array.from({ length: program_count }, (_: any, i: number) => i)

  for (let i = 0; i < allIds.length; i += BATCH_SIZE) {
    const batchIds = allIds.slice(i, i + BATCH_SIZE)

    await Promise.all(batchIds.map(async (programId: number) => {
      try {
        const iconData: number[] = await invoke('load_program_icon', {
          programGuid: programId,
        })

        const blob = new Blob([new Uint8Array(iconData)], { type: 'image/png' })
        const url = URL.createObjectURL(blob)
        program_icons.value.set(programId, url)
      } catch (error: any) {
        console.error(`${t('app.preload_icon_failed')}: ${programId}`, error)
      }
    }))
  }
}

const getIcons = async (keys: Array<number>) => {
  let result: Array<string> = []
  for (let key of keys) {
    if (program_icons.value.has(key)) {
      result.push(program_icons.value.get(key) as string)
    } else {
      let iconData: number[] = await invoke('load_program_icon', { programGuid: key })
      const blob = new Blob([new Uint8Array(iconData)], { type: 'image/png' })
      const url = URL.createObjectURL(blob)
      program_icons.value.set(key, url)
      result.push(url)
    }
  }
  return result
}

const launch_program = async (itemIndex: number, ctrlKey = false, shiftKey = false) => {
  if (parameterSession.value) {
    return
  }

  const currentResults = is_alt_pressed.value ? latest_launch_program.value : searchResults.value
  const selected = currentResults[itemIndex]
  if (!selected) {
    return
  }

  const program_guid = selected[0]

  try {
    const info = await invoke<LaunchTemplateInfoResponse>('get_launch_template_info', {
      programGuid: program_guid,
    })

    if (info.placeholder_count > 0) {
      startParameterSession(program_guid, ctrlKey, shiftKey, info)
      return
    }
  } catch (error) {
    console.warn('Failed to get launch template info, falling back to direct launch:', error)
  }

  await invoke('launch_program', {
    programGuid: program_guid,
    ctrl: ctrlKey,
    shift: shiftKey,
    args: [],
  })
}

const handleItemClick = (itemIndex: number, ctrlKey: boolean) => {
  launch_program(itemIndex, ctrlKey)
}

const initSearchBar = () => {
  searchText.value = ''
  selectedIndex.value = 0
  if (resultsListRef.value?.resultsListRef) {
    resultsListRef.value.resultsListRef.scrollTop = 0
  }
}

const handleClickOutside = () => {
  if (searchBarMenuBuf.value?.isVisible()) {
    searchBarMenuBuf.value?.hideMenu()
  }
  if (resultItemMenuRef.value?.isVisible()) {
    resultItemMenuRef.value?.hideMenu()
  }
}

const focusSearchInput = () => {
  searchBarMenuBuf.value?.hideMenu()
  resultItemMenuRef.value?.hideMenu()
  initSearchBar()
  searchBarRef.value?.focus()
}

const applyTheme = async (isDark: boolean) => {

  is_dark.value = isDark
  await invoke('command_change_tray_icon', { isDark: isDark })
}

const updateTheme = () => {
  const mode = ui_config.value.theme_mode;
  if (mode === 'dark') {
    applyTheme(true);
  } else if (mode === 'light') {
    applyTheme(false);
  } else {
    // System
    if (darkModeMediaQuery.value) {
      applyTheme(darkModeMediaQuery.value.matches);
    }
  }
}

function handleThemeChange(e: MediaQueryListEvent) {
  if (ui_config.value.theme_mode === 'system') {
    applyTheme(e.matches)
  }
}

watch(() => ui_config.value.theme_mode, () => {
  updateTheme()
})

const handleRightArrow = (event: KeyboardEvent) => {
  // Logic to check cursor position would go here if we had access
  // For now, we can just show submenu if we want to support that feature
  // Or we need to expose cursor position from SearchBar
  
  // Assuming we want to show submenu for the selected item
  event.preventDefault()
  showSubmenuForItem(selectedIndex.value)
}

const { handleKeyDown, handleKeyUp, handleBlur } = useShortcuts(
  app_config,
  shortcut_config,
  ui_config,
  isEverythingMode,
  toggleEverythingMode,
  everythingPanelRef,
  resultsListRef,
  resultItemMenuRef,
  searchBarRef,
  searchText,
  selectedIndex,
  is_alt_pressed,
  latest_launch_program,
  searchResults,
  launch_program,
  confirmParameterInput,
  cancelParameterSession,
  parameterSession,
  handleRightArrow
)

const contextResultItemEvent = (index: number, event: MouseEvent) => {
  if (searchBarMenuBuf.value?.isVisible()) {
    searchBarMenuBuf.value?.hideMenu()
  }
  selectedIndex.value = index
  resultItemMenuRef.value?.showMenu({ top: event.clientY, left: event.clientX })
}

const openFolder = async () => {
  await invoke('open_target_folder', { programGuid: searchResults.value[selectedIndex.value][0] })
}

const runTargetProgramWithAdmin = () => {
  launch_program(selectedIndex.value, true, false)
}

const showSubmenuForItem = (index: number) => {
  const selectedItem = document.querySelectorAll('.result-item')[index]
  if (!selectedItem) return

  const rect = selectedItem.getBoundingClientRect()
  resultItemMenuRef.value?.showMenu({ top: rect.top, left: rect.width })
}

let unlisten: Array<UnlistenFn | null> = []

watch(searchText, (newVal: string) => {
  sendSearchText(newVal)
})

watch(parameterSession, async (session: ParameterSession | null) => {
  if (session) {
    await nextTick()
    parameterPanelRef.value?.focus()
  } else {
    await nextTick()
    searchBarRef.value?.focus()
  }
})

watch(is_alt_pressed, async (new_value: boolean) => {
  if (new_value) {
    await get_latest_launch_program()
  }
  await refresh_result_items()
  selectedIndex.value = 0
  if (resultsListRef.value?.resultsListRef) {
    resultsListRef.value.resultsListRef.scrollTop = 0
  }
})

onMounted(async () => {
  darkModeMediaQuery.value = window.matchMedia('(prefers-color-scheme: dark)')
  updateTheme()
  darkModeMediaQuery.value.addEventListener('change', handleThemeChange)

  searchBarRef.value?.focus()
  updateWindow()

  window.addEventListener('click', handleClickOutside)
  unlisten.push(await listen('show_window', () => {
    focusSearchInput()
    is_visible.value = true
  }))
  
  window.addEventListener('wheel', (event) => {
    if (event.ctrlKey) {
      event.preventDefault()
    }
  }, { passive: false })
  
  unlisten.push(await listen('update_search_bar_window', () => {
    updateWindow()
    invoke<string>('command_get_search_status_tip').then((code: string) => status_reason_code.value = code).catch(() => {})
  }))
  
  unlisten.push(await listen('refresh_program_start', () => {
    is_refreshing_dataset.value = true
  }))
  unlisten.push(await listen('refresh_program_end', () => {
    is_refreshing_dataset.value = false
  }))
  unlisten.push(await listen('handle_focus_lost', () => {
    initSearchBar()
    is_visible.value = false
  }))

  const currentWindow = getCurrentWindow()
  windowSize.value = await currentWindow.innerSize()
  scaleFactor.value = await currentWindow.scaleFactor()

  currentWindow.onResized(async ({ payload: size }: any) => {
    scaleFactor.value = await currentWindow.scaleFactor()
    windowSize.value = size
  })
})

onUnmounted(() => {
  window.removeEventListener('click', handleClickOutside)
  for (const listener of unlisten) {
    if (listener) {
      listener()
    }
  }
  darkModeMediaQuery.value?.removeEventListener('change', handleThemeChange)
})
</script>

<style>
/*
这里选择99.85是因为如果选择100%，可能会出现底栏被挡住的情况
比如：如果在屏幕上的高度为532，而缩放比为150%，那么对应这个界面来说，高度为 532 / 1.5 = 354.666...
这个多出来的小数会导致计算错误，从而导致底栏的边框消失，如果让这个显示的界面小一点点，就不会出现这个情况了
*/

html,
body {
  box-sizing: border-box;
  height: 99.85%;
  margin: 0;
  padding: 0;
}

#app {
  box-sizing: border-box;
  height: 100%;
  width: 100%;
}

main {
  height: 100%;
}
</style>

<style scoped>
.launcher-container {
  display: flex;
  padding: 0;
  overflow: hidden;
  outline: none;
  flex-direction: column;
  height: calc(100%);
  width: 100%;
  box-sizing: border-box;
}

.unified-container {
  flex-direction: column;
  overflow: hidden;
  min-height: 0;
  flex-shrink: 0;
}
</style>


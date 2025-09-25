<template>
  <div class="launcher-container" @keydown="handleKeyDown" @keyup="handleKeyUp" @blur="handleBlur" tabindex="0" :style="[program_backgroundStyle,
    !ui_config.use_windows_sys_control_radius ? {
      border: `1px solid ${is_dark ? '#3d3d3d' : '#bdbdbd'}`,
      borderRadius: `${ui_config.window_corner_radius}px`
    } : {}]">
    <div class="unified-container">
      <!-- 搜索栏 -->
      <div class="search-input drag_area"
        :style="{ background: ui_config.search_bar_background_color, height: ui_config.search_bar_height + 'px' }">
        <span class="search-icon drag_area" :style="{
          marginLeft: Math.round(ui_config.search_bar_height * 0.3) + 'px',
          marginRight: Math.round(ui_config.search_bar_height * 0.3) + 'px'
        }">
          <svg viewBox="0 0 1024 1024" class="drag_area" :width="Math.round(ui_config.search_bar_height * 0.4) + 'px'"
            :height="Math.round(ui_config.search_bar_height * 0.4) + 'px'">
            <path fill="#999" class="drag_area"
              d="M795.904 750.72l124.992 124.928a32 32 0 0 1-45.248 45.248L750.656 795.904a416 416 0 1 1 45.248-45.248zM480 832a352 352 0 1 0 0-704 352 352 0 0 0 0 704z" />
          </svg>
        </span>
        <AnimatedInput v-model="searchText" :placeholder="app_config.search_bar_placeholder" ref="searchBarRef"
          @contextmenu.prevent="contextSearchBarEvent"
          :font-size="Math.round(ui_config.search_bar_height * ui_config.search_bar_font_size / 100) + 'px'"
          :color="ui_config.search_bar_font_color" :font-family="ui_config.search_bar_font_family"
          :placeholder-color="ui_config.search_bar_placeholder_font_color" :dynamic="ui_config.search_bar_animate" />
      </div>

      <!-- 二级菜单,外观与另一个菜单保持一致 -->

      <SubMenu ref="searchBarMenuBuf" :itemHeight="ui_config.result_item_height" :windowSize="innerWindowSize"
        :menuItems="searchBarMenuItems" :isDark="is_dark" :cornerRadius="ui_config.window_corner_radius"
        :hoverColor="hover_item_color" :selectedColor="ui_config.selected_item_color"
        :itemFontColor="ui_config.item_font_color" :itemFontSizePercent="ui_config.item_font_size"
        :style="submenu_backgroundStyle"> </SubMenu>

      <!--结果列表 -->
      <div class="results-list" :class="{ 'scroll-mode': isScrollMode }" ref="resultsListRef">
        <div v-for="(item, index) in menuItems" :key="index" class="result-item"
          @click="(event) => handleItemClick(index, event.ctrlKey)" :class="{ 'selected': selectedIndex === index }"
          @contextmenu.prevent="(event) => contextResultItemEvent(index, event)" :style="{
            '--hover-color': hover_item_color,
            '--selected-color': ui_config.selected_item_color,
            height: ui_config.result_item_height + 'px',
          }">
          <div class="icon" :style="{
            width: Math.round(ui_config.result_item_height * 0.6) + 'px',
            height: Math.round(ui_config.result_item_height * 0.6) + 'px',
            marginLeft: Math.round(ui_config.result_item_height * 0.2) + 'px',
            marginRight: Math.round(ui_config.result_item_height * 0.2) + 'px',
          }">
            <img :src="menuIcons[index]" class="custom-image" alt="icon">
          </div>
          <div class="item-info">
            <div class="item-name" v-html="item" :style="{
              fontSize: Math.round(ui_config.result_item_height * ui_config.item_font_size / 100) + 'px',
              fontFamily: ui_config.result_item_font_family,
              color: ui_config.item_font_color
            }"></div>
          </div>
        </div>
      </div>
    </div>
    <!-- 二级菜单 -->
    <SubMenu ref="resultItemMenuRef" :itemHeight="ui_config.result_item_height" :windowSize="innerWindowSize"
      :menuItems="resultSubMenuItems" :isDark="is_dark" :cornerRadius="ui_config.window_corner_radius"
      :hoverColor="hover_item_color" :selectedColor="ui_config.selected_item_color"
      :itemFontColor="ui_config.item_font_color" :itemFontSizePercent="ui_config.item_font_size"
      :style="submenu_backgroundStyle"></SubMenu>


    <!-- 底部状态栏 -->
    <div v-if="ui_config.footer_height > 0" class="footer drag_area"
      :style="{ backgroundColor: ui_config.search_bar_background_color, fontSize: Math.round(ui_config.footer_height * ui_config.footer_font_size / 100) + 'px', fontFamily: ui_config.footer_font_family, }">
      <div class="footer-left">
        <span class="status-text" :style="{ color: ui_config.footer_font_color }">{{
          app_config.tips }}</span>
      </div>
      <div class="footer-center drag_area"></div>
      <div class="footer-right">
        <span class="open-text" :style="{ color: ui_config.footer_font_color }">{{ is_loading_icons ?
          t('app.loading_icons') :
          right_tips
        }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch, onUnmounted } from 'vue'
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core'
import { reduceOpacity } from '../utils/color';
import { AppConfig, default_app_config, default_ui_config, PartialAppConfig, PartialUIConfig, ShortcutConfig, UIConfig, default_shortcut_config, PartialShortcutConfig } from '../api/remote_config_types';
import SubMenu from '../utils/SubMenu.vue';
import { FolderOpened, Refresh, Setting, StarFilled } from '@element-plus/icons-vue';
import { getCurrentWindow } from '@tauri-apps/api/window';
import AnimatedInput from './components/AnimatedInput.vue';
import { useI18n } from 'vue-i18n';
import { initializeLanguage } from '../i18n';

const { t } = useI18n();

const app_config = ref<AppConfig>(default_app_config())
const ui_config = ref<UIConfig>(default_ui_config())
const shortcut_config = ref<ShortcutConfig>(default_shortcut_config())
const searchText = ref('')
const selectedIndex = ref<number>(0)
const searchBarRef = ref<InstanceType<typeof AnimatedInput> | null>(null)
// 结果列表容器的引用
const resultsListRef = ref<HTMLElement | null>(null)
// 搜索的结果
const searchResults = ref<Array<[number, string]>>([]);
// 最近启动的程序
const latest_launch_program = ref<Array<[number, string]>>([]);
// 当前是否按下了alt键
const is_alt_pressed = ref<boolean>(false);
//右下角的提示词
const right_tips = ref<string>(t('app.best_match'));
const menuItems = ref<Array<string>>([]);
const menuIcons = ref<Array<string>>([]);
const program_icons = ref<Map<number, string>>(new Map<number, string>([]));
// 当前的搜索栏是否可见
const is_visible = ref<boolean>(false);
const hover_item_color = computed(() => {
  return reduceOpacity(ui_config.value.selected_item_color, 0.8);
})
const background_picture = ref('');

// 用于检测当前系统是深色模式还是浅色模式
const darkModeMediaQuery = ref<MediaQueryList | null>(null);
const is_dark = ref(false);

// 表示当前是不是正在加载图片
const is_loading_icons = ref<boolean>(false);

// 计算是否启用滚动模式
const isScrollMode = computed(() => {
  const currentResults = is_alt_pressed.value ? latest_launch_program.value : searchResults.value;
  return currentResults.length > app_config.value.scroll_threshold;
});

const scrollModeMaxHeight = computed(() => {
  return `${app_config.value.scroll_threshold * ui_config.value.result_item_height}px`;
});

let unlisten: Array<UnlistenFn | null> = [];

watch(searchText, (newVal) => {
  sendSearchText(newVal)
})

const sendSearchText = async (text: string) => {
  try {
    const results: Array<[number, string]> = await invoke('handle_search_text', { searchText: text });
    searchResults.value = results;
    await refresh_result_items();
    // 搜索时重置滚动位置和选中项
    selectedIndex.value = 0;
    if (resultsListRef.value) {
      resultsListRef.value.scrollTop = 0;
    }
  } catch (error) {
    console.error('Error sending search text to Rust: ', error);
  }
}

// 监测alt键的变化
watch(is_alt_pressed, async (new_value) => {
  // 只要有变化，就调用
  if (new_value) {
    // 如果按下了，则向后端调用
    await get_latest_launch_program();
  }
  // 然后刷新
  await refresh_result_items();
  // Alt键切换时重置滚动位置和选中项
  selectedIndex.value = 0;
  if (resultsListRef.value) {
    resultsListRef.value.scrollTop = 0;
  }
})

const get_latest_launch_program = async () => {
  const results: Array<[number, string]> = await invoke('command_get_latest_launch_program');
  latest_launch_program.value = results;
  await refresh_result_items();
}

const refresh_result_items = async () => {
  if (!is_alt_pressed.value) {
    menuItems.value = searchResults.value.map(([_, item]) => item);
    let keys = searchResults.value.map(([key, _]) => key);
    menuIcons.value = await getIcons(keys);
    right_tips.value = t('app.best_match');
  } else {
    menuItems.value = latest_launch_program.value.map(([_, item]) => item);
    let keys = latest_launch_program.value.map(([key, _]) => key);
    menuIcons.value = await getIcons(keys);
    right_tips.value = t('app.recent_open');
  }
}

const searchBarMenuBuf = ref<InstanceType<typeof SubMenu> | null>(null);
const searchBarMenuItems = computed(() => [{ name: t('menu.open_settings'), icon: Setting, action: () => { openSettingsWindow() } },
{ name: t('menu.refresh_database'), icon: Refresh, action: () => { refreshDataset() } }]);

const contextSearchBarEvent = (event: MouseEvent) => {
  if (resultItemMenuRef.value?.isVisible) {
    resultItemMenuRef.value?.hideMenu()
  }
  searchBarMenuBuf.value?.showMenu({ top: event.clientY, left: event.clientX });
}

// 打开设置窗口的方法
const openSettingsWindow = () => {
  // 调用后端或其他逻辑来打开设置窗口
  invoke('show_setting_window')
    .then(() => {
      console.log('Settings window opened.');
    })
    .catch((error) => {
      console.error('Failed to open settings window:', error);
    });
};

// 刷新程序库
const refreshDataset = async () => {
  console.log(t('app.start_refresh'));
  await invoke('hide_window');
  await invoke('refresh_program');
  updateWindow();
}

// 用于程序在更新相应内容
const updateWindow = async () => {
  console.log("updateWindow");
  try {
    const background_picture_data = await invoke<number[]>('get_background_picture');
    const program_count = invoke<number>('get_program_count');
    const data = await invoke<[PartialAppConfig, PartialUIConfig, PartialShortcutConfig]>('update_search_bar_window');
    app_config.value = { ...app_config.value, ...data[0] }
    ui_config.value = { ...ui_config.value, ...data[1] }
    shortcut_config.value = { ...shortcut_config.value, ...data[2] }
    await initializeLanguage(app_config.value.language);
    const elements = document.querySelectorAll('.drag_area');
    if (app_config.value.is_enable_drag_window) {
      elements.forEach(element => {
        element.setAttribute('data-tauri-drag-region', 'true');
      });
    } else {
      elements.forEach(element => {
        element.removeAttribute('data-tauri-drag-region');
      });
    }
    const blob = new Blob([new Uint8Array(background_picture_data)], { type: 'image/png' });
    const url = URL.createObjectURL(blob);

    background_picture.value = url;

    // 先清空图标缓存，确保数据库更新后图标与程序名保持一致
    program_icons.value.forEach(url => URL.revokeObjectURL(url));
    program_icons.value.clear();

    // 如果没有这个，那么就会导致在没有更新完成时，结果栏也是空的，这样不好看，所以提前发送一次搜索文本
    if (!is_visible || searchText.value.length == 0) {
      await sendSearchText('');
    }
    await startPreloadResource(await program_count).then(async () => {
      is_loading_icons.value = false;
      // 如果没有这个，那么可能会导致图标加载不正确（显示是空的），加了以后会再次搜索，从而显示正确的图标
      if (!is_visible || searchText.value.length == 0) {
        await sendSearchText('');
      }
    });

  } catch (error) {
    console.error('Error in updateWindow:', error);
  }
}

const startPreloadResource = async (program_count: number) => {
  is_loading_icons.value = true;
  const BATCH_SIZE = 100; // 增大批次大小以提升效率

  // 释放旧资源
  program_icons.value.forEach(url => URL.revokeObjectURL(url));
  program_icons.value.clear();

  // 创建所有programId的数组（0到program_count-1）
  const allIds = Array.from({ length: program_count }, (_, i) => i);

  // 分批并发加载
  for (let i = 0; i < allIds.length; i += BATCH_SIZE) {
    const batchIds = allIds.slice(i, i + BATCH_SIZE);

    await Promise.all(batchIds.map(async (programId) => {
      try {

        const iconData: number[] = await invoke('load_program_icon', {
          programGuid: programId
        });

        const blob = new Blob([new Uint8Array(iconData)], { type: 'image/png' });
        const url = URL.createObjectURL(blob);
        program_icons.value.set(programId, url);
      } catch (error) {
        console.error(`${t('app.preload_icon_failed')}: ${programId}`, error);
      }
    }));
  }
}

const getIcons = async (keys: Array<number>) => {
  let result: Array<string> = [];
  for (let key of keys) {
    if (program_icons.value.has(key)) {
      result.push(program_icons.value.get(key) as string);
    } else {
      let iconData: number[] = await invoke('load_program_icon', { programGuid: key });
      const blob = new Blob([new Uint8Array(iconData)], { type: 'image/png' });
      const url = URL.createObjectURL(blob);
      program_icons.value.set(key, url);
      result.push(url);
    }
  }
  return result;
}


// 处理选中项目的函数，现在接收 ctrlKey 参数
const launch_program = (itemIndex: number, ctrlKey = false, shiftKey = false) => {
  const program_guid = is_alt_pressed.value ? latest_launch_program.value[itemIndex][0] : searchResults.value[itemIndex][0]
  invoke('launch_program', { programGuid: program_guid, ctrl: ctrlKey, shift: shiftKey });
  // 这里可以添加实际的处理逻辑
}

// 定义操作类型
enum ActionType {
  MOVE_DOWN,
  MOVE_UP,
  MOVE_RIGHT,
  MOVE_LEFT,
  CONFIRM,
  ESCAPE
}

// 窗口失去了焦点
const handleBlur = () => {
  // console.log('Window blurred'); // 调试用
  is_alt_pressed.value = false;
}

// 键盘按键起来
const handleKeyUp = (event: KeyboardEvent) => {
  console.log("up");
  console.log(event);
  if (event.key === 'Alt') {
    is_alt_pressed.value = false;
  }
}

// 阻止一些与程序无关的webview快捷键
/**
 * 阻止 WebView 的默认快捷键行为
 * @param {KeyboardEvent} event 
 */
const preventDefaultWebViewShortcuts = (event: KeyboardEvent) => {
  // 1. 阻止刷新
  // F5 和 Ctrl+R, Ctrl+Shift+R
  if (event.key === 'F5' || (event.ctrlKey && event.key.toLowerCase() === 'r')) {
    event.preventDefault();
  }

  // 2. 阻止打印
  // Ctrl+P
  if (event.ctrlKey && event.key.toLowerCase() === 'p') {
    event.preventDefault();
  }

  // 3. 阻止页面缩放
  // Ctrl + (加号), Ctrl + (减号), Ctrl + 0
  if (event.ctrlKey && ['=', '-', '0'].includes(event.key)) {
    event.preventDefault();
  }

  // 4. 阻止其他可能不需要的快捷键
  // Ctrl+F (查找), Ctrl+S (保存)
  if (event.ctrlKey && ['f', 's'].includes(event.key.toLowerCase())) {
    event.preventDefault();
  }
};


// 键盘事件处理函数
const handleKeyDown = async (event: KeyboardEvent) => {
  preventDefaultWebViewShortcuts(event);
  const isMenuVisible = resultItemMenuRef.value?.isVisible() || false;
  // 获取真实的 input 元素
  const inputElement = searchBarRef.value?.realInputRef;
  if (event.key === 'Alt') {
    is_alt_pressed.value = true;
    event.preventDefault();
  }

  // 检查是否匹配快捷键
  const matchShortcut = (shortcutConfig: any): boolean => {
    return event.key.toLowerCase() === shortcutConfig.key.toLowerCase() &&
      event.ctrlKey === shortcutConfig.ctrl &&
      //event.altKey === shortcutConfig.alt &&
      event.shiftKey === shortcutConfig.shift &&
      event.metaKey === shortcutConfig.meta;
  };

  // 处理方向键和快捷键
  if (event.key === 'ArrowDown' || matchShortcut(shortcut_config.value.arrow_down)) {
    event.preventDefault();
    handleAction(ActionType.MOVE_DOWN, isMenuVisible);
    return;
  }

  if (event.key === 'ArrowUp' || matchShortcut(shortcut_config.value.arrow_up)) {
    event.preventDefault();
    handleAction(ActionType.MOVE_UP, isMenuVisible);
    return;
  }

  if (event.key === 'ArrowRight' || matchShortcut(shortcut_config.value.arrow_right)) {
    const isAtEnd = inputElement && (inputElement.selectionStart === searchText.value.length);
    if (!isMenuVisible && isAtEnd && document.activeElement === inputElement) {
      // 只有在光标在末尾时，才阻止默认行为并执行我们的操作（比如打开子菜单）
      event.preventDefault();
      handleAction(ActionType.MOVE_RIGHT, isMenuVisible);
    }
    return;
  }

  if (event.key === 'ArrowLeft' || matchShortcut(shortcut_config.value.arrow_left)) {
    if (isMenuVisible) {
      event.preventDefault();
      handleAction(ActionType.MOVE_LEFT, isMenuVisible);
    }
    return;
  }

  // 处理特殊键
  if (event.key === 'Enter' || (event.key === ' ' && app_config.value.space_is_enter)) {
    event.preventDefault();
    handleAction(ActionType.CONFIRM, isMenuVisible, event.ctrlKey, event.shiftKey);
    return;
  }

  if (event.key === 'Escape') {
    handleAction(ActionType.ESCAPE, isMenuVisible);
    return;
  }
};

// 滚动到选中项的函数
const scrollToSelectedItem = () => {
  if (!resultsListRef.value || !isScrollMode.value) return;
  
  const container = resultsListRef.value;
  const itemHeight = ui_config.value.result_item_height;
  const selectedItemTop = selectedIndex.value * itemHeight;
  const selectedItemBottom = selectedItemTop + itemHeight;
  const containerScrollTop = container.scrollTop;
  const containerHeight = container.clientHeight;
  const containerScrollBottom = containerScrollTop + containerHeight;

  let targetScrollTop = null;

  // 如果选中项在可视区域上方，滚动到选中项顶部
  if (selectedItemTop < containerScrollTop) {
    targetScrollTop = selectedItemTop;
  }
  // 如果选中项在可视区域下方，滚动到选中项底部可见
  else if (selectedItemBottom > containerScrollBottom) {
    targetScrollTop = selectedItemBottom - containerHeight;
  }

  // 使用smooth滚动动画
  if (targetScrollTop !== null) {
    container.scrollTo({
      top: targetScrollTop,
      behavior: 'smooth'
    });
  }
};

// 处理各种操作的函数
const handleAction = (
  action: ActionType,
  isMenuVisible: boolean,
  ctrlKey: boolean = false,
  shiftKey: boolean = false
) => {
  switch (action) {
    case ActionType.MOVE_DOWN:
      if (isMenuVisible) {
        resultItemMenuRef.value?.selectNext();
      } else {
        const currentResults = is_alt_pressed.value ? latest_launch_program.value : searchResults.value;
        selectedIndex.value = (selectedIndex.value + 1) % Math.min(currentResults.length, app_config.value.search_result_count);
        scrollToSelectedItem();
      }
      break;

    case ActionType.MOVE_UP:
      if (isMenuVisible) {
        resultItemMenuRef.value?.selectPrevious();
      } else {
        const currentResults = is_alt_pressed.value ? latest_launch_program.value : searchResults.value;
        const maxIndex = Math.min(currentResults.length, app_config.value.search_result_count);
        selectedIndex.value = (selectedIndex.value - 1 + maxIndex) % maxIndex;
        scrollToSelectedItem();
      }
      break;

    case ActionType.MOVE_RIGHT:
      if (!isMenuVisible) {
        handleRightArrow(new KeyboardEvent('keydown'));
      }
      break;

    case ActionType.MOVE_LEFT:
      if (isMenuVisible) {
        resultItemMenuRef.value?.hideMenu();
      }
      break;

    case ActionType.CONFIRM:
      if (isMenuVisible) {
        resultItemMenuRef.value?.selectCurrent();
      } else {
        launch_program(selectedIndex.value, ctrlKey, shiftKey);
      }
      break;

    case ActionType.ESCAPE:
      if ((searchText.value.length === 0 && !isMenuVisible) ||
        app_config.value.is_esc_hide_window_priority) {
        invoke('hide_window').catch(console.error);
      } else {
        if (isMenuVisible) {
          resultItemMenuRef.value?.hideMenu();
        } else {
          searchText.value = '';
        }
      }
      break;
  }
};

// 处理点击项目，现在传递 ctrlKey 状态
const handleItemClick = (itemIndex: number, ctrlKey = false) => {
  // 传递 ctrlKey 状态到 handle 函数
  launch_program(itemIndex, ctrlKey)
}

const initSearchBar = () => {
  searchText.value = '';
  selectedIndex.value = 0;
  // 重置滚动位置
  if (resultsListRef.value) {
    resultsListRef.value.scrollTop = 0;
  }
}

// 点击页面其他地方时隐藏自定义菜单
const handleClickOutside = () => {
  if (searchBarMenuBuf.value?.isVisible) {
    searchBarMenuBuf.value?.hideMenu()
  }
  if (resultItemMenuRef.value?.isVisible) {
    resultItemMenuRef.value?.hideMenu()
  }
}

const focusSearchInput = () => {
  searchBarMenuBuf.value?.hideMenu();
  resultItemMenuRef.value?.hideMenu();
  initSearchBar();
  if (searchBarRef.value) {
    searchBarRef.value.focus();
  }
}

const submenu_backgroundStyle = computed(() => ({
  backgroundColor: `${ui_config.value.program_background_color}`,
}));

const program_backgroundStyle = computed(() => ({
  backgroundColor: (ui_config.value.blur_style !== 'None' && ui_config.value.use_windows_sys_control_radius === true)
    ? 'transparent'
    : ui_config.value.program_background_color,
  backgroundImage: `linear-gradient(rgba(255, 255, 255, ${1 - ui_config.value.background_opacity}), rgba(255, 255, 255, ${1 - ui_config.value.background_opacity})), url(${background_picture.value})`,
  backgroundSize: `${ui_config.value.background_size}`,
  backgroundPosition: `${ui_config.value.background_position}`,
  backgroundRepeat: `${ui_config.value.background_repeat}`,
  backgroundClip: 'content-box',
}));

const applyTheme = async (isDark: boolean) => {
  // 这里可以根据实际主题需求设置颜色变量
  console.log(`主题变更为: ${isDark ? t('app.theme_change_dark') : t('app.theme_change_light')}`);
  is_dark.value = isDark;
  await invoke('command_change_tray_icon', { isDark: isDark })
}

// 主题变化处理函数
function handleThemeChange(e: MediaQueryListEvent) {
  applyTheme(e.matches);
}

const handleRightArrow = (event: KeyboardEvent) => {
  const input = searchBarRef.value;
  if (!input) return;

  // 获取光标位置
  const cursorPos = input.cursorPosition;
  const textLength = searchText.value.length;
  if (cursorPos !== textLength) {
    // 允许默认的光标移动
    return;
  }

  event.preventDefault();
  showSubmenuForItem(selectedIndex.value)
};


const resultItemMenuRef = ref<InstanceType<typeof SubMenu> | null>(null);
const resultSubMenuItems = computed(() => [{ name: t('app.open_file_location'), icon: FolderOpened, action: () => { openFolder() } },
{ name: t('app.run_as_admin'), icon: StarFilled, action: () => { runTargetProgramWithAdmin() } }]);

const contextResultItemEvent = (index: number, event: MouseEvent) => {
  if (searchBarMenuBuf.value?.isVisible) {
    searchBarMenuBuf.value?.hideMenu()
  }
  selectedIndex.value = index;
  resultItemMenuRef.value?.showMenu({ top: event.clientY, left: event.clientX });
}

const openFolder = async () => {
  await invoke('open_target_folder', { programGuid: searchResults.value[selectedIndex.value][0] })
  // todo:打开对应的文件夹
}

const runTargetProgramWithAdmin = () => {
  launch_program(selectedIndex.value, true, false)
}

// 显示子菜单
const showSubmenuForItem = (index: number) => {
  const selectedItem = document.querySelectorAll('.result-item')[index];
  if (!selectedItem) return;

  const rect = selectedItem.getBoundingClientRect();

  // 设置子菜单位置 - 在选中项目的右侧显示
  resultItemMenuRef.value?.showMenu({ top: rect.top, left: rect.width });
};

const innerWindowSize = computed(() => {
  return {
    width: Math.round(windowSize.value.width / scaleFactor.value),
    height: Math.round(windowSize.value.height / scaleFactor.value)
  }
})

const windowSize = ref<{
  width: number;
  height: number;
}>({ width: 800, height: 800 });

const scaleFactor = ref<number>(1);

// 组件挂载后自动聚焦容器以接收键盘事件
onMounted(async () => {
  // 初始化主题
  darkModeMediaQuery.value = window.matchMedia('(prefers-color-scheme: dark)');
  applyTheme(darkModeMediaQuery.value.matches);
  // 添加主题变化监听
  darkModeMediaQuery.value.addEventListener('change', handleThemeChange);


  if (searchBarRef.value) {
    searchBarRef.value.focus()
  }
  updateWindow();
  // 在更新完图标后再进行显示的更新

  window.addEventListener('click', handleClickOutside);
  unlisten.push(await listen('show_window', () => {
    focusSearchInput();
    is_visible.value = true;
  }));
  // 阻止通过鼠标滚轮进行缩放
  window.addEventListener('wheel', (event) => {
    if (event.ctrlKey) {
      event.preventDefault();
    }
  }, { passive: false });
  unlisten.push(await listen('update_search_bar_window', () => {
    updateWindow();
  }));
  unlisten.push(await listen('handle_focus_lost', () => {
    initSearchBar();
    is_visible.value = false;
  }));

  // 获取当前窗口
  const currentWindow = getCurrentWindow();

  // 获取窗口大小
  windowSize.value = await currentWindow.innerSize();
  scaleFactor.value = await currentWindow.scaleFactor();

  // 监听窗口大小变化
  currentWindow.onResized(async ({ payload: size }) => {
    scaleFactor.value = await currentWindow.scaleFactor();
    windowSize.value = size;
  });
})

onUnmounted(() => {
  window.removeEventListener('click', handleClickOutside);
  for (let item of unlisten) {
    if (item) {
      item();
    }
  }
  program_icons.value.forEach((url) => URL.revokeObjectURL(url));
});

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

.search-input {
  box-sizing: border-box;
  display: flex;
  align-items: center;
  border-bottom: 1px solid rgba(0, 0, 0, 0.05);
  flex-shrink: 0;
  flex-grow: 1;
  /* 确保它能填满剩余空间 */
  min-width: 0;
}

.search-icon {
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.results-list {
  overflow-y: auto;
  min-height: 0;
  scrollbar-width: thin;
  scrollbar-color: rgba(0, 0, 0, 0.2) transparent;
}

/* 滚动模式样式 */
.results-list.scroll-mode {
  max-height: v-bind(scrollModeMaxHeight);
  overflow-y: auto;
}

/* 自定义滚动条样式 */
.results-list::-webkit-scrollbar {
  width: 6px;
}

.results-list::-webkit-scrollbar-track {
  background: transparent;
}

.results-list::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.2);
  border-radius: 3px;
}

.results-list::-webkit-scrollbar-thumb:hover {
  background: rgba(0, 0, 0, 0.4);
}

.result-item {
  display: flex;
  align-items: center;
  cursor: pointer;
  transition: background-color 0.2s;
  flex-shrink: 0;
}


.result-item:hover {
  background-color: var(--hover-color);
}

.result-item.selected {
  background-color: var(--selected-color);
}


.icon {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.custom-image {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  border-radius: 6px;
  image-rendering: -webkit-optimize-contrast;
  transform: translateZ(0);
}

.item-info {
  display: flex;
  align-items: center;
  min-width: 0;
  overflow: hidden;
  height: 100%;
}

.item-name {
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  width: 100%;
  line-height: normal;
}

mark {
  background-color: transparent;
  color: inherit;
  font-weight: 700;
  padding: 0;
}

.footer {
  box-sizing: border-box;
  flex: 1;
  display: flex;
  align-items: center;
  border-top: 1px solid rgba(0, 0, 0, 0.05);
}

.footer-left {
  margin-left: 16px;
  flex-shrink: 0;
}

.footer-center {
  flex-grow: 1;
}

.footer-right {
  margin-right: 16px;
  flex-shrink: 0;
}

.status-text,
.open-text {
  color: #666;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
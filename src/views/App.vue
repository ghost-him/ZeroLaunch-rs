<template>
  <div class="launcher-container" @keydown="handleKeyDown" tabindex="0" :style="backgroundStyle">
    <div class="unified-container">
      <!-- 搜索栏 -->
      <div class="search-input"
        :style="{ background: search_bar_data.search_bar_background_color, height: search_bar_data.search_bar_height + 'px' }">
        <span class="search-icon">
          <svg viewBox="0 0 1024 1024" width="26" height="26">
            <path fill="#999"
              d="M795.904 750.72l124.992 124.928a32 32 0 0 1-45.248 45.248L750.656 795.904a416 416 0 1 1 45.248-45.248zM480 832a352 352 0 1 0 0-704 352 352 0 0 0 0 704z" />
          </svg>
        </span>
        <input v-model="searchText" :placeholder="search_bar_data.search_bar_placeholder" class="input-field"
          ref="searchBarRef" @contextmenu.prevent="showContextMenu" :style="{
            fontSize: search_bar_data.search_bar_font_size + 'rem',
            color: search_bar_data.search_bar_font_color
          }">
      </div>

      <!-- 上下文菜单 -->
      <div v-if="isContextMenuVisible" class="custom-context-menu" :style="{
        top: `${contextMenuPosition.y}px`,
        left: `${contextMenuPosition.x}px`
      }" @click.stop>
        <div class="context-menu">
          <div class="context-menu-item" @click="handleContextMenuSelect('openSettings')">
            打开设置窗口
          </div>
          <div class="context-menu-item" @click="handleContextMenuSelect('refreshDataset')">
            刷新程序数据
          </div>
        </div>
      </div>

      <!-- 结果列表 -->
      <div class="results-list">
        <div v-for="(item, index) in menuItems" :key="index" class="result-item"
          @click="(event) => handleItemClick(index, event.ctrlKey)" :class="{ 'selected': selectedIndex === index }"
          :style="{
            '--hover-color': hover_item_color,
            '--selected-color': search_bar_data.selected_item_color,
            height: search_bar_data.result_item_height + 'px',
          }">
          <div class="icon">
            <img :src="menuIcons[index]" class="custom-image" alt="icon">
          </div>
          <div class="item-info">
            <div class="item-name" v-html="item" :style="{
              fontSize: search_bar_data.item_font_size + 'rem',
              color: search_bar_data.item_font_color
            }"></div>
          </div>
        </div>
      </div>
    </div>

    <!-- 底部状态栏 -->
    <div v-if="search_bar_data.footer_height > 0" class="footer"
      :style="{ backgroundColor: search_bar_data.search_bar_background_color, height: search_bar_data.footer_height + 'px' }">
      <div class="footer-left">
        <span class="status-text">{{ search_bar_data.tips }}</span>
      </div>
      <div class="footer-right">
        <span class="open-text">{{ '打开' }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch, onUnmounted } from 'vue'
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core'
import { reduceOpacity } from '../utils/color';

interface SearchBarUpdate {
  search_bar_placeholder: string;
  selected_item_color: string;
  item_font_color: string,
  search_bar_font_color: string,
  search_bar_background_color: string,
  item_font_size: number,
  search_bar_font_size: number,
  tips: string
  search_bar_height: number,
  result_item_height: number,
  footer_height: number,
  result_item_count: number,
}

const search_bar_data = ref<SearchBarUpdate>(
  {
    search_bar_placeholder: 'Hello, ZeroLaunch!',
    selected_item_color: '#e3e3e3cc',
    item_font_color: '#000000',
    search_bar_font_color: '#333333',
    search_bar_background_color: '#FFFFFF00',
    item_font_size: 1.3,
    search_bar_font_size: 2.0,
    tips: 'ZeroLaunch-rs',
    search_bar_height: 65,
    result_item_height: 62,
    footer_height: 42,
    result_item_count: 4
  }
);

const searchText = ref('la')
const selectedIndex = ref<number>(0)
const searchBarRef = ref<HTMLInputElement | null>(null)
const searchResults = ref<Array<[number, string]>>([]);
const menuItems = ref<Array<string>>([]);
const menuIcons = ref<Array<string>>([]);
const program_icons = ref<Map<number, string>>(new Map<number, string>([]));
const isContextMenuVisible = ref(false);
const contextMenuPosition = ref({ x: 0, y: 0 });
const hover_item_color = computed(() => {
  return reduceOpacity(search_bar_data.value.selected_item_color, 0.8);
})
const background_picture = ref('');

// 用于检测当前系统是深色模式还是浅色模式
const darkModeMediaQuery = ref<MediaQueryList | null>(null);

let unlisten: Array<UnlistenFn | null> = [];

watch(searchText, (newVal) => {
  sendSearchText(newVal)
})

const sendSearchText = async (text: string) => {
  console.log(hover_item_color)
  try {
    const results: Array<[number, string]> = await invoke('handle_search_text', { searchText: text });
    searchResults.value = results;
    menuItems.value = results.map(([_, item]) => item);
    let keys = results.map(([key, _]) => key);
    menuIcons.value = await getIcons(keys);
  } catch (error) {
    console.error('Error sending search text to Rust: ', error);
  }
}

// 处理菜单项选择
const handleContextMenuSelect = (index: string) => {
  if (index === 'openSettings') {
    openSettingsWindow();
  } else if (index === 'refreshDataset') {
    refreshDataset();
  }
  isContextMenuVisible.value = false;
};

// 显示上下文菜单
const showContextMenu = (event: MouseEvent) => {
  event.preventDefault();
  isContextMenuVisible.value = true;
  contextMenuPosition.value = { x: event.clientX, y: event.clientY };
};

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
  console.log("开始刷新");
  await invoke('hide_window');
  await invoke('refresh_program');
  updateWindow();
}

// 用于程序在更新相应内容
const updateWindow = async () => {
  console.log("updateWindow");
  try {
    sendSearchText('');
    const background_picture_data = await invoke<number[]>('get_background_picture');
    const program_count = invoke<number>('get_program_count');
    const data = await invoke<SearchBarUpdate>('update_search_bar_window');
    search_bar_data.value = data;

    const blob = new Blob([new Uint8Array(background_picture_data)], { type: 'image/png' });
    const url = URL.createObjectURL(blob);

    background_picture.value = url;
    await startPreloadResource(await program_count);
  } catch (error) {
    console.error('Error in updateWindow:', error);
  }
}

const startPreloadResource = async (program_count: number) => {
  const BATCH_SIZE = 10; // 增大批次大小以提升效率

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
        console.error(`预加载图标失败: ${programId}`, error);
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
  console.log("向后端调用");
  invoke('launch_program', { programGuid: searchResults.value[itemIndex][0], ctrl: ctrlKey, shift: shiftKey });
  // 这里可以添加实际的处理逻辑
}

// 处理键盘导航
const handleKeyDown = async (event: KeyboardEvent) => {

  switch (event.key) {
    case 'ArrowDown':
      event.preventDefault()
      selectedIndex.value = (selectedIndex.value + 1) % search_bar_data.value.result_item_count
      break
    case 'ArrowUp':
      event.preventDefault()
      selectedIndex.value = (selectedIndex.value - 1 + search_bar_data.value.result_item_count) % search_bar_data.value.result_item_count
      break
    case 'Enter':
      event.preventDefault()
      // 传递 ctrlKey 状态到 handle 函数
      launch_program(selectedIndex.value, event.ctrlKey, event.shiftKey)
      break
    case 'j':
      if (event.ctrlKey) {
        event.preventDefault()
        selectedIndex.value = (selectedIndex.value + 1) % search_bar_data.value.result_item_count
      }
      break
    case 'k':
      if (event.ctrlKey) {
        event.preventDefault()
        selectedIndex.value = (selectedIndex.value - 1 + search_bar_data.value.result_item_count) % search_bar_data.value.result_item_count
      }
      break
    case 'Escape':
      if (searchText.value.length === 0) {
        await invoke('hide_window');
      } else {
        searchText.value = '';
      }
      break
  }
}

// 处理点击项目，现在传递 ctrlKey 状态
const handleItemClick = (itemIndex: number, ctrlKey = false) => {
  // 传递 ctrlKey 状态到 handle 函数
  launch_program(itemIndex, ctrlKey)
}

const initSearchBar = () => {
  searchText.value = '';
  selectedIndex.value = 0;
}

// 点击页面其他地方时隐藏自定义菜单
const handleClickOutside = () => {
  if (isContextMenuVisible.value) {
    isContextMenuVisible.value = false;
  }
}

const focusSearchInput = () => {
  initSearchBar();
  if (searchBarRef.value) {
    searchBarRef.value.focus();
  }
  isContextMenuVisible.value = false;
}

const backgroundStyle = computed(() => ({
  backgroundImage: `url(${background_picture.value})`,
  backgroundSize: 'cover',
  backgroundPosition: 'center',
  backgroundRepeat: 'no-repeat',
  backgroundClip: 'content-box',
}));

const applyTheme = async (isDark: boolean) => {
  // 这里可以根据实际主题需求设置颜色变量
  console.log(`主题变更为: ${isDark ? '深色' : '浅色'}`);
  await invoke('command_change_tray_icon', { isDark: isDark })
}

// 主题变化处理函数
function handleThemeChange(e: MediaQueryListEvent) {
  applyTheme(e.matches);
}

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
  updateWindow()
  window.addEventListener('click', handleClickOutside);
  unlisten.push(await listen('show_window', () => {
    focusSearchInput();
  }));
  unlisten.push(await listen('update_search_bar_window', () => {
    updateWindow();
  }));
  unlisten.push(await listen('handle_focus_lost', () => {
    initSearchBar();
  }));
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

<style scoped>
.launcher-container {
  border-radius: 12px;
  border: 1px solid #b2abab;
  background-color: white;
  padding: 0;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
  overflow: hidden;
  outline: none;
  display: flex;
  flex-direction: column;
  height: 100%;
  width: 100%;
  box-sizing: border-box;
}

.unified-container {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
  min-height: 0;
}

.search-input {
  display: flex;
  align-items: center;
  border-bottom: 1px solid rgba(0, 0, 0, 0.05);
  flex-shrink: 0;
}

.input-field {
  flex: 1;
  border: none;
  outline: none;
  font-weight: 600;
  background: transparent;
  width: 100%;
  padding: 0;
  margin: 0;
  height: 100%;
  line-height: normal;
}

.search-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  margin-left: 20px;
  margin-right: 20px;
  flex-shrink: 0;
}

.results-list {
  overflow-y: auto;
  flex: 1;
  min-height: 0;
  scrollbar-width: thin;
  scrollbar-color: rgba(0, 0, 0, 0.2) transparent;
}

.result-item {
  display: flex;
  align-items: center;
  cursor: pointer;
  transition: background-color 0.2s;
}


.result-item:hover {
  background-color: var(--hover-color);
}

.result-item.selected {
  background-color: var(--selected-color);
}


.icon {
  width: 36px;
  height: 36px;
  margin-left: 18px;
  margin-right: 18px;
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
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  overflow: hidden;
  justify-content: center;
  height: 100%;
}

.item-name {
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  justify-content: center;
  line-height: normal;
}

mark {
  background-color: transparent;
  color: inherit;
  font-weight: 700;
  padding: 0;
}

.footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-top: 1px solid rgba(0, 0, 0, 0.05);
  flex-shrink: 0;
}

.footer-left {
  margin-left: 16px;
  flex: 1;
  min-width: 0;
}

.footer-right {
  display: flex;
  align-items: center;
  margin-right: 16px;
}

.status-text,
.open-text {
  color: #666;
  font-size: 14px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.custom-context-menu {
  position: fixed;
  z-index: 1000;
}

.context-menu {
  background-color: white;
  border-radius: 4px;
  box-shadow: 0 2px 12px 0 rgba(0, 0, 0, 0.1);
  padding: 5px 0;
  min-width: 150px;
  max-width: 300px;
}

.context-menu-item {
  padding: 8px 20px;
  cursor: pointer;
  font-size: 14px;
  color: #606266;
  transition: background-color 0.3s;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.context-menu-item:hover {
  background-color: #f5f7fa;
  color: #409eff;
}
</style>
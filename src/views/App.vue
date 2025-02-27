<template>
  <div class="launcher-container" @keydown="handleKeyDown" tabindex="0" :style="{ ...backgroundStyle }">
    <div class="unified-container">
      <div class="search-input">
        <span class="search-icon">
          <svg viewBox="0 0 1024 1024" style="width:24px;height:24px;">
            <path fill="#999"
              d="M795.904 750.72l124.992 124.928a32 32 0 0 1-45.248 45.248L750.656 795.904a416 416 0 1 1 45.248-45.248zM480 832a352 352 0 1 0 0-704 352 352 0 0 0 0 704z" />
          </svg>
        </span>
        <input v-model="searchText" :placeholder="placeholder" class="input-field" ref="searchBarRef"
          @contextmenu.prevent="showContextMenu">
      </div>

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


      <div class="results-list">
        <div v-for="(item, index) in menuItems" class="result-item"
          @click="(event) => handleItemClick(index, event.ctrlKey)" :class="{ 'selected': selectedIndex === index }">
          <div class="icon">
            <img :src="menuIcons[index]" class="custom-image">
          </div>
          <div class="item-info">
            <div class="item-name" v-html="item"></div>
          </div>
        </div>
      </div>
    </div>

    <div class="footer">
      <div class="footer-left">
        <span class="status-text">{{ statusText }}</span>
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

const searchText = ref('la')
const selectedIndex = ref(0)
const searchBarRef = ref<HTMLInputElement | null>(null)
const statusText = ref('准备就绪')
const searchResults = ref<Array<[number, string]>>([]);
const menuItems = ref<Array<string>>([]);
const menuIcons = ref<Array<string>>([]);
const program_icons = ref<Map<number, string>>(new Map<number, string>([]));
let placeholder = ref('请输入搜索内容');
const isContextMenuVisible = ref(false);
const contextMenuPosition = ref({ x: 0, y: 0 });
const resultItemCount = ref<number>(1);
const selected_item_color = ref('#d55d1d');
const item_font_color = ref('#ffeeee');
const background_picture = ref('');
let unlisten: Array<UnlistenFn | null> = [];
interface SearchBarInit {
  result_item_count: number;
}
interface SearchBarUpdate {
  search_bar_placeholder: string;
  selected_item_color: string;
  item_font_color: string,
}

watch(searchText, (newVal) => {
  sendSearchText(newVal)
})

const sendSearchText = async (text: String) => {
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
  invoke('hide_window');
  await invoke('refresh_program');
  updateWindow();
}

// 用于程序在更新相应内容
const updateWindow = async () => {
  console.log("updateWindow");
  try {
    const background_picture_data = await invoke<number[]>('get_background_picture');
    const program_count = invoke<number>('get_program_count');
    const data = await invoke<SearchBarUpdate>('update_search_bar_window');
    placeholder.value = data.search_bar_placeholder;
    selected_item_color.value = data.selected_item_color;
    item_font_color.value = data.item_font_color;

    const blob = new Blob([new Uint8Array(background_picture_data)], { type: 'image/png' });
    const url = URL.createObjectURL(blob);

    background_picture.value = url;
    await startPreloadResource(await program_count);
  } catch (error) {
    console.error('Error in updateWindow:', error);
  }
}

const startPreloadResource = async (program_count: number) => {
  const BATCH_SIZE = 10; // 每批加载的图标数量
  program_icons.value.clear(); // 每次加载时，都要清空原来的内容
  // 分批次处理程序ID
  for (let i = 0; i < program_count; i += BATCH_SIZE) {
    const batchStart = i;
    const batchEnd = Math.min(i + BATCH_SIZE, program_count);
    const batch = Array.from({ length: batchEnd - batchStart }, (_, index) => batchStart + index);

    // 并发加载当前批次的所有图标
    await Promise.all(batch.map(async (programId) => {
      try {
        const iconData: number[] = await invoke('load_program_icon', { programGuid: programId });
        const blob = new Blob([new Uint8Array(iconData)], { type: 'image/png' });
        const url = URL.createObjectURL(blob);
        program_icons.value.set(programId, url); // 确保 program_icons 已定义
      } catch (error) {
        console.error(`Failed to preload icon for program ${programId}:`, error);
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
const launch_program = (itemIndex: number, ctrlKey = false) => {
  console.log("向后端调用");
  invoke('launch_program', { programGuid: searchResults.value[itemIndex][0], isAdminRequired: ctrlKey });
  // 这里可以添加实际的处理逻辑
}

// 处理键盘导航
const handleKeyDown = (event: KeyboardEvent) => {

  switch (event.key) {
    case 'ArrowDown':
      event.preventDefault()
      selectedIndex.value = (selectedIndex.value + 1) % resultItemCount.value
      break
    case 'ArrowUp':
      event.preventDefault()
      selectedIndex.value = (selectedIndex.value - 1 + resultItemCount.value) % resultItemCount.value
      break
    case 'Enter':
      event.preventDefault()
      // 传递 ctrlKey 状态到 handle 函数
      launch_program(selectedIndex.value, event.ctrlKey)
      break
    case 'j':
      if (event.ctrlKey) {
        event.preventDefault()
        selectedIndex.value = (selectedIndex.value + 1) % resultItemCount.value
      }
      break
    case 'k':
      if (event.ctrlKey) {
        event.preventDefault()
        selectedIndex.value = (selectedIndex.value - 1 + resultItemCount.value) % resultItemCount.value
      }
      break
  }
}

// 处理点击项目，现在传递 ctrlKey 状态
const handleItemClick = (itemIndex: number, ctrlKey = false) => {
  // 传递 ctrlKey 状态到 handle 函数
  launch_program(itemIndex, ctrlKey)
}

const initWindow = async () => {
  const initValue = await invoke<SearchBarInit>('initialize_search_window');
  resultItemCount.value = initValue.result_item_count;
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
}

const backgroundStyle = computed(() => ({
  backgroundImage: `url(${background_picture.value})`,
  backgroundSize: 'cover',
  backgroundPosition: 'center',
  backgroundRepeat: 'no-repeat',
  backgroundClip: 'content-box',
}));


// 组件挂载后自动聚焦容器以接收键盘事件
onMounted(async () => {
  if (searchBarRef.value) {
    searchBarRef.value.focus()
  }
  initWindow()
  updateWindow()
  sendSearchText('');
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
  border: #b2abab solid 1px;
  background: white;
  padding: 0;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
  overflow: hidden;
  outline: none;
  /* 移除聚焦时的轮廓 */
}

.unified-container {
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.search-input {
  display: flex;
  align-items: center;
  padding: 13px 16px 12px 16px;

  border-bottom: 1px solid rgba(0, 0, 0, 0.05);
}

.input-field {
  flex: 1;
  border: none;
  outline: none;
  font-size: 2em;
  font-weight: 600;
  background: transparent;
  color: #333;
}

.search-icon {
  display: flex;
  align-items: center;
  color: #333;
  margin-right: 12px;
}

.results-list {
  max-height: 95vh;
  overflow-y: auto;
  background-color: rgba(255, 255, 255, 0);
}

.result-item {
  display: flex;
  align-items: center;
  padding: 13px 16px;
  cursor: pointer;
  transition: background-color 0.2s;
}

.result-item:hover {
  background-color: rgba(232, 240, 254, 0.5);
}

.result-item.selected {
  background-color: rgba(232, 240, 254, 0.8);
}

.icon {
  width: 36px;
  height: 36px;
  margin-right: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.custom-image {
  width: 100%;
  height: 100%;
  object-fit: contain;
  border-radius: 6px;
}

.item-info {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.item-name {
  font-size: 1.3rem;
  font-weight: 500;
  color: #333;
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
  padding: 12px 16px;
  border-top: 1px solid rgba(0, 0, 0, 0.05);
}

.footer-left {
  flex: 1;
}

.footer-right {
  display: flex;
  align-items: center;
}

.status-text {
  color: #666;
  font-size: 14px;
}

.open-text {
  color: #666;
  font-size: 14px;
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
}

.context-menu-item {
  padding: 8px 20px;
  cursor: pointer;
  font-size: 14px;
  color: #606266;
  transition: background-color 0.3s;
}

.context-menu-item:hover {
  background-color: #f5f7fa;
  color: #409eff;
}
</style>
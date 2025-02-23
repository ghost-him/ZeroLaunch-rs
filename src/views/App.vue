<template>
  <div v-if="scaledWindowSize[0] && scaledWindowSize[1]" class="container" :style="{

    width: `${scaledWindowSize[0]}px`,
    height: `${scaledWindowSize[1]}px`
  }">
    <el-input ref="searchInputRef" v-model="searchText" :placeholder="placeholder" class="search-input" autosize
      @keyup.enter="handleEnterPress" @contextmenu.prevent="showContextMenu" :style="{
        width: `${scaledItemSize[0]}px`,
        height: `${scaledItemSize[1]}px`,
        fontSize: `${scaledFontSize}px`,
        fontColor: `${fontColor}`,
        fontFamily: `${fontFamily}`
      }">
      <template #prefix>
        <el-icon style="padding-left: 20px;">
          <Search />
        </el-icon>
      </template>
    </el-input>

    <div v-if="isContextMenuVisible" class="custom-context-menu" :style="{
      top: `${contextMenuPosition.y}px`,
      left: `${contextMenuPosition.x}px`
    }" @click.stop>
      <el-menu @select="handleContextMenuSelect" class="context-menu">
        <el-menu-item index="openSettings" class="context-menu">打开设置窗口</el-menu-item>
        <el-menu-item index="refreshDataset" class="context-menu">刷新程序数据</el-menu-item>
        <!-- 可以在这里添加更多的菜单项 -->
      </el-menu>
    </div>


    <el-menu :default-active="activeIndex" class="menu-list round-border" @select="handleSelectMouse" :style="{
      width: `${scaledItemSize[0]}px`,
      height: `${scaledItemSize[1] * resultItemCount}px`, ...backgroundStyle
    }">
      <el-menu-item v-for="(item, index) in menuItems" :key="index" :index="String(index)"
        class="menu-item round-border" :style="{
          width: `${scaledItemSize[0]}px`,
          height: `${scaledItemSize[1]}px`,
          fontSize: `${scaledFontSize * 0.7}px`
        }" @click="launch_program(index)" @contextmenu.prevent>
        <div class="common-layout">
          <el-container>
            <el-aside class="icon-container">
              <el-image :style="{
                width: `${scaledItemSize[1] * 0.8}px`,
                height: `${scaledItemSize[1] * 0.8}px`,
              }" :src="menuIcons[index]" fit="contain" />
            </el-aside>
            <el-main>{{ item }}</el-main>
          </el-container>
        </div>
      </el-menu-item>
    </el-menu>
  </div>
  <div v-else>
    <p>加载中...</p>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed, onUnmounted, watch } from 'vue';
import { Search } from '@element-plus/icons-vue';
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { calculateColors } from "../utils/color"

const searchText = ref('');
const activeIndex = ref('0');
const menuItems = ref();
const resultItemCount = ref(4);
const menuIcons = ref<Array<string>>([]);
const searchResults = ref<Array<[number, string]>>([]);
const windowSize = ref<[number, number]>([0, 0]);
const itemSize = ref<[number, number]>([0, 0]);
const scaleFactor = ref<number>(1.0);

const fontFamily = ref('Arial, sans-serif');
const fontSize = ref<number>(0);
const fontColor = ref('#333333');

const selected_item_color = ref('#d55d1d');
const item_font_color = ref('#ffeeee');

const background_picture = ref('');
const program_icons = ref<Map<number, string>>(new Map<number, string>([]));

let placeholder = ref('请输入搜索内容');
const searchInputRef = ref<HTMLInputElement | null>(null);
let unlisten: Array<UnlistenFn | null> = [];
// 新增的状态用于自定义右键菜单
const isContextMenuVisible = ref(false);
const contextMenuPosition = ref({ x: 0, y: 0 });

const isCtrlPressed = ref(false);


const sendSearchText = async (text: string) => {
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

// 使用计算属性动态计算缩放后的尺寸
const scaledWindowSize = computed(() => {
  const factor = scaleFactor.value || 1;
  return windowSize.value.map(size => size / factor) as [number, number];
});

const scaledItemSize = computed(() => {
  const factor = scaleFactor.value || 1;
  return itemSize.value.map(size => size / factor) as [number, number];
});

const scaledFontSize = computed(() => {
  const factor = scaleFactor.value || 1;
  return fontSize.value / factor;
})

const computed_selected_item_color = computed(() => {
  return calculateColors(selected_item_color.value).selected;
})

const computed_no_selected_item_color = computed(() => {
  return calculateColors(selected_item_color.value).nonSelected;
})

// (鼠标选择)
const handleSelectMouse = (index: string) => {
  activeIndex.value = index;
}

watch(searchText, (newValue) => {
  sendSearchText(newValue);
});

const handleKeyDown = (event: KeyboardEvent) => {
  if (event.key === 'Control') {
    isCtrlPressed.value = true;
  }
  if (event.key === 'ArrowUp' || (event.ctrlKey && event.key === 'k')) {
    event.preventDefault();
    selectPreviousItem();
  } else if (event.key === 'ArrowDown' || (event.ctrlKey && event.key === 'j')) {
    event.preventDefault();
    selectNextItem();
  } else if (event.key === 'Escape') {
    event.preventDefault();
    pressedESC();
  }
};

const handleKeyUp = (event: KeyboardEvent) => {
  if (event.key === 'Control') {
    isCtrlPressed.value = false;
  }
};

const selectPreviousItem = () => {
  const currentIndex = parseInt(activeIndex.value);
  const newIndex = (currentIndex - 1 + menuItems.value.length) % menuItems.value.length;
  activeIndex.value = newIndex.toString();
};

const selectNextItem = () => {
  const currentIndex = parseInt(activeIndex.value);
  const newIndex = (currentIndex + 1) % menuItems.value.length;
  activeIndex.value = newIndex.toString();
};

const focusSearchInput = () => {
  initSearchBar();
  if (searchInputRef.value) {
    searchInputRef.value.focus();
  }
}

const pressedESC = () => {
  activeIndex.value = '0';
  if (searchText.value === '') {
    // 隐藏窗口
    invoke('hide_window');
  } else {
    // 清空文字
    searchText.value = '';
  }
}


// 新增的方法：显示自定义右键菜单
const showContextMenu = (event: MouseEvent) => {
  event.preventDefault();
  isContextMenuVisible.value = true;
  contextMenuPosition.value = { x: event.clientX, y: event.clientY };
};

// 刷新程序库
const refreshDataset = async () => {
  console.log("开始刷新");
  invoke('hide_window');
  await invoke('refresh_program');
  updateWindow();
}

// 处理自定义菜单项选择
const handleContextMenuSelect = (index: string) => {
  if (index === 'openSettings') {
    openSettingsWindow();
  } else if (index === 'refreshDataset') {
    refreshDataset();
  }
  isContextMenuVisible.value = false;
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

// 点击页面其他地方时隐藏自定义菜单
const handleClickOutside = () => {
  if (isContextMenuVisible.value) {
    isContextMenuVisible.value = false;
  }
}

interface SearchBarInit {
  window_size: [number, number];
  item_size: [number, number];
  window_scale_factor: number;
  result_item_count: number;
}

interface SearchBarUpdate {
  search_bar_placeholder: string;
  selected_item_color: string;
  item_font_color: string,
}

// 用于程序在一开始初始化
const initWindow = async () => {
  const initValue = await invoke<SearchBarInit>('initialize_search_window');
  resultItemCount.value = initValue.result_item_count;
  windowSize.value = initValue.window_size;
  itemSize.value = initValue.item_size;
  fontSize.value = itemSize.value[1] / 2;
  scaleFactor.value = initValue.window_scale_factor
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

defineExpose({ updateWindow })

// 用于初始化搜索栏和快捷键的状态(当成功启动一个程序时，或者搜索栏被隐藏时被触发)
const initSearchBar = () => {
  isCtrlPressed.value = false;
  searchText.value = '';
  activeIndex.value = '0';
}

const handleEnterPress = () => {
  launch_program(parseInt(activeIndex.value))
}


const launch_program = (index: number) => {
  const ctrlPressed = isCtrlPressed.value;
  console.log(`Launching program for item ${searchResults.value[index][0]}, Ctrl key pressed: ${ctrlPressed}`);

  invoke('launch_program', { programGuid: searchResults.value[index][0], isAdminRequired: ctrlPressed });
};

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

const backgroundStyle = computed(() => ({
  backgroundImage: `url(${background_picture.value})`,
  backgroundSize: 'cover',
  backgroundPosition: 'center',
  backgroundRepeat: 'no-repeat',
  backgroundClip: 'content-box',
  padding: '10px',
}));

// 在组件挂载时添加事件监听器
onMounted(async () => {
  initWindow();
  updateWindow();
  sendSearchText('');
  focusSearchInput();
  window.addEventListener('keydown', handleKeyDown);
  window.addEventListener('keyup', handleKeyUp);
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

});

// 在组件卸载时移除事件监听器
onUnmounted(() => {
  window.removeEventListener('keydown', handleKeyDown);
  window.removeEventListener('keyup', handleKeyUp);
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
html,
body,
#app {
  height: 100%;
  margin: 0;
  padding: 0;
}


/* 自定义右键菜单样式 */
.custom-context-menu {
  color: #000 !important;
  position: fixed;
  z-index: 1000;
  background-color: #fff !important;
  border: 1px solid #dcdfe6;
  box-shadow: 0 2px 12px 0 rgba(0, 0, 0, 0.1);
  border-radius: 4px;
}

.context-menu {
  color: #000 !important;
  background-color: #fff !important;
  min-width: 150px;
}

.round-border {
  border-radius: 10px;
}

.container {
  padding: 0;
  display: flex;
  flex-direction: column;
  align-items: stretch;
  /* 修改为 stretch 以确保子元素宽度填满容器 */
  justify-content: flex-start;
  /* 从顶部开始排列子元素 */
  background-size: contain;
  /* 使用 contain 确保图片完整显示 */
  background-position: center;
  background-repeat: no-repeat;
  /* 防止图片重复 */
}

.menu-list {
  margin-top: 20px;
  overflow: hidden;
  border: 1px solid var(--el-border-color);
  padding: 0 !important;
  min-height: 0 !important;
}

.menu-item {
  /* border: 1px solid var(--el-border-color); */
  overflow: hidden;

}



.common-layout {
  height: 100%;
  width: 100%;
  padding: 0;
}


:deep(.el-menu-item.is-active) {
  color: v-bind(item_font_color);
  background-color: v-bind(computed_selected_item_color) !important;
}

:deep(.el-menu-item:hover) {
  background-color: v-bind(computed_no_selected_item_color) !important;
}

:deep(.el-menu-item) {
  color: v-bind(item_font_color);
  background-color: rgba(0, 0, 0, 0);
}

/* 添加自定义样式来覆盖 Element Plus 默认样式 */
:deep(.el-input__wrapper) {
  /* box-shadow: none !important; */
  padding: 0 !important;
}

:deep(.el-input__inner) {
  font-family: v-bind(fontFamily);
  font-size: v-bind(scaledFontSize) + 'px';
  color: v-bind(fontColor);

  height: 100%;
}

:deep(.el-input__inner::placeholder) {
  color: v-bind(fontColor);
  opacity: 0.4;
}



.common-layout {
  height: 100%;
  width: 100%;
  padding: 0;
  display: flex;
  align-items: center;
  /* 垂直居中 */
}

.icon-container {
  display: flex;
  justify-content: center;
  align-items: center;
  width: auto !important;
  min-width: 50px;
  margin-right: 10px;
}

:deep(.el-main) {
  padding: 0;
  display: flex;
  align-items: center;
}

.el-image {
  margin: 0;
  padding: 0;
  display: flex;
  justify-content: center;
  align-items: center;
  width: auto !important;
  min-width: 50px;
  margin-right: 10px;
}

/* Adjust el-aside to have a fixed width */
:deep(.el-aside) {
  width: auto !important;
}

/* Ensure the container takes full height */
:deep(.el-container) {
  height: 100%;
}
</style>

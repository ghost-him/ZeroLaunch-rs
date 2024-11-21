<template>
  <div v-if="scaledWindowSize[0] && scaledWindowSize[1]" class="container" :style="{
    backgroundImage: `url(${backgroundImage})`,
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
        <el-menu-item index="openSettings">打开设置窗口</el-menu-item>
        <el-menu-item index="refreshDataset">刷新程序数据</el-menu-item>
        <!-- 可以在这里添加更多的菜单项 -->
      </el-menu>
    </div>


    <el-menu :default-active="activeIndex" class="menu-list round-border" @select="handleSelectMouse" :style="{
      width: `${scaledItemSize[0]}px`,
      height: `${scaledItemSize[1] * 4}px`
    }">
      <el-menu-item v-for="(item, index) in menuItems" :key="index" :index="String(index)"
        class="menu-item round-border" :style="{
          width: `${scaledItemSize[0]}px`,
          height: `${scaledItemSize[1]}px`
        }" @click="launch_program(index)">
        <div class="common-layout">
          <el-container>
            <el-aside width="50">
              <el-image :style="{
                width: `${scaledItemSize[1]}px`,
                height: `${scaledItemSize[1]}px`
              }" :src="`data:image/png;base64,${menuIcons[index]}`" fit="cover" />
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

const backgroundImage = ref('https://example.com/default-background.jpg');
const searchText = ref('');
const activeIndex = ref('0');
const menuItems = ref(['hello world', 'hello world', 'hello world', 'hello world']);
const menuIcons = ref<Array<string>>([]);
const searchResults = ref<Array<[number, string]>>([]);
const windowSize = ref<[number, number]>([0, 0]);
const itemSize = ref<[number, number]>([0, 0]);
const scaleFactor = ref<number>(1.0);

const fontFamily = ref('Arial, sans-serif');
const fontSize = ref<number>(0);
const fontColor = ref('#333333');

const program_icons = ref<Map<number, string>>(new Map<number, string>([]));

let placeholder = ref('请输入搜索内容');
const searchInputRef = ref<HTMLInputElement | null>(null);
let unlisten1: UnlistenFn | null = null;
let unlisten2: UnlistenFn | null = null;
let unlisten3: UnlistenFn | null = null;
// 新增的状态用于自定义右键菜单
const isContextMenuVisible = ref(false);
const contextMenuPosition = ref({ x: 0, y: 0 });

const isCtrlPressed = ref(false);

/*
// 更新字体的样式
const updateInputStyle = (newFont: string, newSize: number, newColor: string) => {
  fontFamily.value = newFont;
  fontSize.value = newSize;
  fontColor.value = newColor;
};
*/

const sendSearchText = async (text: string) => {
  try {
    const results: Array<[number, string]> = await invoke('handle_search_text', { searchText: text });
    searchResults.value = results;
    menuItems.value = results.slice(0, 4).map(([_, item]) => item);
    let keys = results.slice(0, 4).map(([key, _]) => key);
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

// 图片URL
const url = 'https://fuss10.elemecdn.com/e/5d/4a731a90594a4af544c0c25941171jpeg.jpeg';

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
const refreshDataset = () => {
  invoke('refresh_program');
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
}

interface SearchBarUpdate {
  search_bar_placeholder: string;
}

// 用于程序在一开始初始化
const initWindow = async () => {
  const initValue = await invoke<SearchBarInit>('init_search_bar_window');

  windowSize.value = initValue.window_size;
  itemSize.value = initValue.item_size;
  fontSize.value = itemSize.value[1] / 2;
  scaleFactor.value = initValue.window_scale_factor
  console.log(initValue);
}

// 用于程序在更新相应内容
const updateWindow = async () => {
  const data = await invoke<SearchBarUpdate>('update_search_bar_window');
  placeholder.value = data.search_bar_placeholder;
}
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
  initSearchBar()
};

const getIcons = async (keys: Array<number>) => {
  let result: Array<string> = new Array(keys.length);
  let requireIndices: Array<[number, number]> = [];

  for (let i = 0; i < keys.length; i++) {
    let key = keys[i];
    if (program_icons.value.has(key)) {
      result[i] = program_icons.value.get(key) as string;
    } else {
      requireIndices.push([i, key]);
      result[i] = ''; // Initialize with empty string
    }
  }

  if (requireIndices.length > 0) {
    let requiredKeys = requireIndices.map(index => index[1]);
    let get_icon: Array<string> = await invoke('load_program_icon', { programGuid: requiredKeys });

    for (let i = 0; i < requireIndices.length; i++) {
      result[requireIndices[i][0]] = get_icon[i];
      program_icons.value.set(requireIndices[i][1], get_icon[i])
    }
  }

  return result;
}

// 在组件挂载时添加事件监听器
onMounted(async () => {
  initWindow();
  updateWindow();
  window.addEventListener('keydown', handleKeyDown);
  window.addEventListener('keyup', handleKeyUp);
  window.addEventListener('click', handleClickOutside);
  unlisten1 = await listen('show_window', () => {
    focusSearchInput();
  });
  unlisten2 = await listen('update_search_bar_window', () => {
    updateWindow();
  })
  unlisten3 = await listen('handle_focus_lost', () => {
    initSearchBar();
  })

});

// 在组件卸载时移除事件监听器
onUnmounted(() => {
  window.removeEventListener('keydown', handleKeyDown);
  window.removeEventListener('keyup', handleKeyUp);
  window.removeEventListener('click', handleClickOutside);
  if (unlisten1) {
    unlisten1();
  }
  if (unlisten2) {
    unlisten2();
  }
  if (unlisten3) {
    unlisten3();
  }
});
</script>

<style scoped>
/* 自定义右键菜单样式 */
.custom-context-menu {
  position: fixed;
  z-index: 1000;
  background-color: #fff;
  border: 1px solid #dcdfe6;
  box-shadow: 0 2px 12px 0 rgba(0, 0, 0, 0.1);
  border-radius: 4px;
}

.context-menu {
  min-width: 150px;
}

.round-border {
  border-radius: 10px;
}

.container {
  padding: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  background-size: cover;
  background-position: center;
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
  color: #fff;
  background-color: #f56c6c !important;
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
</style>

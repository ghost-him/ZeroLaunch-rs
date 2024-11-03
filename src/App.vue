<template>
  <div
    v-if="scaledWindowSize[0] && scaledWindowSize[1]"
    class="container"
    :style="{
      backgroundImage: `url(${backgroundImage})`,
      width: `${scaledWindowSize[0]}px`,
      height: `${scaledWindowSize[1]}px`
    }"
  >
    <el-input
      v-model="searchText"
      :placeholder="placeholder"
      class="search-input"
      autosize
      :style="{
        width: `${scaledItemSize[0]}px`,
        height: `${scaledItemSize[1]}px`,
        fontSize: `${scaledFontSize}px`,
        fontColor: `${fontColor}`,
        fontFamily: `${fontFamily}`
      }"
    >
      <template #prefix>
        <el-icon><Search /></el-icon>
      </template>
    </el-input>

    <el-menu
      :default-active="activeIndex"
      class="menu-list round-border"
      @select="handleSelectMouse"
      :style="{
        width: `${scaledItemSize[0]}px`,
        height: `${scaledItemSize[1] * 4}px`
      }"
    >
      <el-menu-item
        v-for="(item, index) in menuItems"
        :key="index"
        :index="String(index)"
        class="menu-item round-border"
        :style="{
          width: `${scaledItemSize[0]}px`,
          height: `${scaledItemSize[1]}px`
        }"
      >
        <div class="common-layout">
          <el-container>
            <el-aside width="50">
              <el-image
                :style="{
                  width: `${scaledItemSize[1]}px`,
                  height: `${scaledItemSize[1]}px`
                }"
                :src="url"
                fit="cover"
              />
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
import { ref, onMounted, computed, onUnmounted } from 'vue';
import { Search } from '@element-plus/icons-vue';
import { invoke } from "@tauri-apps/api/core";

const backgroundImage = ref('https://example.com/default-background.jpg');
const searchText = ref('');
const activeIndex = ref('0');
const menuItems = ref(['hello world', 'hello world', 'hello world', 'hello world']);
const windowSize = ref<[number, number]>([0, 0]);
const itemSize = ref<[number, number]>([0, 0]);
const scaleFactor = ref<number>(1.0);

const fontFamily = ref('Arial, sans-serif');
const fontSize = ref<number>(0);
const fontColor = ref('#333333');
const placeholder = ref('请输入搜索内容');

// 更新字体的样式
const updateInputStyle = (newFont: string, newSize: number, newColor: string) => {
  fontFamily.value = newFont;
  fontSize.value = newSize;
  fontColor.value = newColor;
};

// 更新占位符文本的方法
const updatePlaceholder = (newPlaceholder: string) => {
  placeholder.value = newPlaceholder;
};

// 获取窗口大小
const getWindowSize = async () => {
  try {
    windowSize.value = await invoke('get_window_size');
    console.log('Window size:', windowSize.value);
  } catch (error) {
    console.error('Error getting window size:', error);
  }
};

// 获取项大小
const getItemSize = async () => {
  try {
    itemSize.value = await invoke('get_item_size');
    fontSize.value = itemSize.value[1] / 2;
    console.log('Item size:', itemSize.value);
  } catch (error) {
    console.error('Error getting item size:', error);
  }
};

// 获取缩放因子
const getScaleFactor = async () => {
  try {
    scaleFactor.value = await invoke('get_window_scale_factor');
    console.log('Scale factor:', scaleFactor.value);
  } catch (error) {
    console.error('Error getting window scale factor:', error);
  }
};

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


const handleKeyDown = (event: KeyboardEvent) => {
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

const pressedESC = () => {
  activeIndex.value = '0';
  if (searchText.value === '') {
    // 隐藏窗口
  } else {
    // 清空文字
    searchText.value = '';
  }
}

// 在组件挂载时添加事件监听器
onMounted(() => {
  getScaleFactor();
  getWindowSize();
  getItemSize();
  window.addEventListener('keydown', handleKeyDown);
});

// 在组件卸载时移除事件监听器
onUnmounted(() => {
  window.removeEventListener('keydown', handleKeyDown);
});
</script>

<style scoped>
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
}

:deep(.el-input__inner) {
  font-family: v-bind(fontFamily);
  font-size: v-bind(scaledFontSize) + 'px';
  color: v-bind(fontColor);
}

:deep(.el-input__inner::placeholder) {
  color: v-bind(fontColor);
  opacity: 0;
}
</style>

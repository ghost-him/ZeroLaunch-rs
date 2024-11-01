<template>
  <div class="container" :style="{ backgroundImage: `url(${backgroundImage})` }">
    <el-input
      v-model="searchText"
      placeholder="请输入搜索内容"
      class="search-input"
    >
      <template #prefix>
        <el-icon><Search /></el-icon>
      </template>
    </el-input>
    
    <el-menu
      :default-active="activeIndex"
      class="menu-list"
      @select="handleSelect"
    >
      <el-menu-item v-for="(item, index) in menuItems" :key="index" :index="String(index)" class="menu-item">
        <div class="common-layout">
          <el-container>
            <el-aside width="50">
              <el-image style="width: 50px; height: 50px" :src="url" :fit="cover" />
            </el-aside>
            <el-main>{{ item }}</el-main>
          </el-container>
        </div>
      </el-menu-item>
    </el-menu>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import { Search } from '@element-plus/icons-vue'

const backgroundImage = ref('https://example.com/default-background.jpg')
const searchText = ref('')
const activeIndex = ref('0')
const menuItems = ref(['hello world', 'hello world', 'hello world', 'hello world'])
const url =
  'https://fuss10.elemecdn.com/e/5d/4a731a90594a4af544c0c25941171jpeg.jpeg'
const handleSelect = (index) => {
  activeIndex.value = index
}
</script>

<style scoped>
.container {
  display: flex;
  flex-direction: column;
  align-items: center;
  min-height: 100vh;
  padding: 20px;
  background-size: cover;
  background-position: center;
}

.search-input {
  width: 300px;
  height: 50px;
  margin-bottom: 20px;
}

.menu-list {
  width: 300px;
  border-radius: 4px;
  overflow: hidden;
  border: 1px solid var(--el-border-color);
}

.menu-item {
  border-radius: 4px;
  /* border: 1px solid var(--el-border-color); */
  overflow: hidden;
}

:deep(.el-menu-item.is-active) {
  color: #fff;
  background-color: #f56c6c !important;
}
</style>

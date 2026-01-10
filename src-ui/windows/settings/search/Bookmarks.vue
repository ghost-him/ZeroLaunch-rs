<template>
  <div class="settings-page">
    <!-- Unsaved Changes Tip -->
    <el-alert
      type="info"
      :closable="false"
      style="margin-bottom: 15px;"
      :title="t('settings.search.bookmarks.unsaved_changes_tip')"
    />
    
    <div class="path-config-container">
      <!-- Left: Bookmark Source List -->
      <div class="path-list-section">
        <div class="section-header">
          <h3>{{ t('settings.search.bookmarks.source_list') }}</h3>
          <span style="font-size: 12px; color: grey;">{{ bookmarkSources.length }} {{ t('settings.search.bookmarks.items') }}</span>
        </div>

        <el-scrollbar>
          <div 
            v-for="(source, index) in bookmarkSources" 
            :key="index" 
            class="path-item"
            :class="{ 'active': selectedIndex === index, 'disabled': !source.enabled }" 
            @click="selectSource(index)"
          >
            <div class="path-info">
              <div class="path-name">
                <el-icon class="mr-2"><Collection /></el-icon>
                {{ source.name }}
              </div>
              <el-tooltip 
                effect="dark"
                :content="source.bookmarks_path" 
                placement="top-start" 
                :show-after="500"
              >
                <div class="path-text">
                  {{ source.bookmarks_path }}
                </div>
              </el-tooltip>
            </div>
            <div class="path-actions">
              <el-switch 
                v-model="source.enabled" 
                @change="saveConfig"
                @click.stop
                size="small"
              />
              <el-button 
                type="danger" 
                size="small" 
                circle 
                @click.stop="removePath(index)"
              >
                <el-icon><Delete /></el-icon>
              </el-button>
            </div>
          </div>
          
          <div v-if="bookmarkSources.length === 0" style="text-align: center; padding: 20px; color: grey;">
             {{ t('settings.search.bookmarks.no_sources') }}
          </div>
        </el-scrollbar>

        <el-button 
          type="primary" 
          @click="detectAndAdd" 
          class="add-path-btn" 
          style="margin-top: 10px; width: calc(100% - 20px); align-self: center;"
          :loading="detecting"
        >
          <el-icon style="margin-right: 5px;"><MagicStick /></el-icon> 
          {{ t('settings.search.bookmarks.auto_detect') }}
        </el-button>
      </div>

      <!-- Right: Settings for selected source -->
      <div class="path-detail-section" v-if="selectedIndex !== null && bookmarkSources[selectedIndex]">
        <div class="detail-form">
          <div class="form-row">
            <h3 style="margin: 0; margin-bottom: 10px;">{{ t('settings.search.bookmarks.settings') }}</h3>
          </div>

          <div class="form-row">
            <div class="form-label">{{ t('settings.search.bookmarks.browser_name') }}:</div>
            <el-input 
              v-model="bookmarkSources[selectedIndex].name" 
              @change="saveConfig"
            >
              <template #prefix>
                <el-icon><Collection /></el-icon>
              </template>
            </el-input>
          </div>

          <div class="form-row">
            <div class="form-label">{{ t('settings.search.bookmarks.path') }}:</div>
            <el-input 
              v-model="bookmarkSources[selectedIndex].bookmarks_path" 
              readonly
            >
              <template #prefix>
                <el-icon><Document /></el-icon>
              </template>
            </el-input>
          </div>

          <div class="form-row">
            <div class="form-label">{{ t('settings.search.bookmarks.enabled') }}:</div>
            <el-switch 
              v-model="bookmarkSources[selectedIndex].enabled" 
              @change="saveConfig"
            />
          </div>

          <!-- Bookmark List Section -->
          <div class="bookmark-list-section">
            <div class="section-header" style="margin-top: 10px; margin-bottom: 10px;">
              <div style="display: flex; align-items: center; gap: 8px;">
                <h4>{{ t('settings.search.bookmarks.bookmark_list') }}</h4>
                <span style="font-size: 11px; color: var(--el-text-color-secondary); font-weight: normal;">
                  {{ t('settings.search.bookmarks.sorted_by_url_hint') }}
                </span>
              </div>
              <span style="font-size: 12px; color: grey;">
                {{ rawBookmarks.length }} {{ t('settings.search.bookmarks.items') }}
              </span>
            </div>

            <div v-if="loadingBookmarks" style="text-align: center; padding: 20px;">
              <el-icon class="is-loading"><Loading /></el-icon>
            </div>

            <el-scrollbar v-else style="flex: 1;">
              <div 
                v-for="(bookmark, bIndex) in sortedBookmarks" 
                :key="bIndex"
                class="bookmark-item"
                :class="{ 'excluded': isBookmarkExcluded(bookmark.url) }"
              >
                <el-checkbox 
                  :model-value="!isBookmarkExcluded(bookmark.url)"
                  @change="(val: boolean) => toggleBookmarkExcluded(bookmark.url, !val)"
                  style="margin-right: 10px; flex-shrink: 0;"
                />
                <div class="bookmark-info">
                  <div class="bookmark-title">
                    {{ getBookmarkDisplayTitle(bookmark) }}
                    <span v-if="hasCustomTitle(bookmark.url)" class="custom-badge">
                      {{ t('settings.search.bookmarks.custom') }}
                    </span>
                  </div>
                  <div class="bookmark-url">{{ bookmark.url }}</div>
                </div>
                <el-button 
                  type="primary" 
                  size="small" 
                  link
                  @click="openEditDialog(bookmark)"
                  style="flex-shrink: 0;"
                >
                  <el-icon><Edit /></el-icon>
                </el-button>
              </div>

              <div v-if="rawBookmarks.length === 0" style="text-align: center; padding: 20px; color: grey;">
                {{ t('settings.search.bookmarks.no_bookmarks') }}
              </div>
            </el-scrollbar>
          </div>
        </div>
      </div>
      <div class="path-detail-section empty-state" v-else>
         <el-empty :description="t('settings.search.bookmarks.select_source')" />
      </div>
    </div>

    <!-- Edit Bookmark Dialog -->
    <el-dialog 
      v-model="editDialogVisible" 
      :title="t('settings.search.bookmarks.edit_bookmark')"
      width="500px"
    >
      <div v-if="editingBookmark">
        <div class="form-row">
          <div class="form-label">URL:</div>
          <el-input :model-value="editingBookmark.url" readonly />
        </div>
        <div class="form-row">
          <div class="form-label">{{ t('settings.search.bookmarks.original_title') }}:</div>
          <el-input :model-value="editingBookmark.title" readonly />
        </div>
        <div class="form-row">
          <div class="form-label">{{ t('settings.search.bookmarks.custom_title') }}:</div>
          <el-input 
            v-model="editingCustomTitle" 
            :placeholder="t('settings.search.bookmarks.custom_title_placeholder')"
            clearable
          />
        </div>
        <div class="form-row">
          <div class="form-label">{{ t('settings.search.bookmarks.excluded') }}:</div>
          <el-switch v-model="editingExcluded" />
        </div>
      </div>
      <template #footer>
        <el-button @click="editDialogVisible = false">{{ t('settings.search.bookmarks.cancel') }}</el-button>
        <el-button type="primary" @click="saveBookmarkOverride">{{ t('settings.search.bookmarks.save') }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { useRemoteConfigStore } from '../../../stores/remote_config';
import { BookmarkSourceConfig, BookmarkOverride } from '../../../api/remote_config_types';
import { 
  Delete, 
  MagicStick, 
  Collection, 
  Document, 
  Edit,
  Loading
} from '@element-plus/icons-vue';
import { ElMessage } from 'element-plus';

const { t } = useI18n();
const remoteConfigStore = useRemoteConfigStore();

const bookmarkSources = ref<BookmarkSourceConfig[]>([]);
const selectedIndex = ref<number | null>(null);
const detecting = ref(false);

// 书签列表相关
interface RawBookmark {
  title: string;
  url: string;
}

const rawBookmarks = ref<RawBookmark[]>([]);
const sortedBookmarks = computed(() => {
  return [...rawBookmarks.value].sort((a, b) => a.url.localeCompare(b.url));
});
const loadingBookmarks = ref(false);

// 编辑对话框相关
const editDialogVisible = ref(false);
const editingBookmark = ref<RawBookmark | null>(null);
const editingCustomTitle = ref('');
const editingExcluded = ref(false);

interface BrowserInfo {
  name: string;
  bookmarks_path: string;
}

// 获取覆盖配置
const getOverrides = (): BookmarkOverride[] => {
  return remoteConfigStore.config.bookmark_loader_config?.overrides || [];
};

// 查找特定 URL 的覆盖配置
const findOverride = (url: string): BookmarkOverride | undefined => {
  return getOverrides().find(o => o.url === url);
};

// 检查书签是否被排除
const isBookmarkExcluded = (url: string): boolean => {
  const override = findOverride(url);
  return override?.excluded ?? false;
};

// 检查书签是否有自定义标题
const hasCustomTitle = (url: string): boolean => {
  const override = findOverride(url);
  return !!(override?.custom_title && override.custom_title.trim());
};

// 获取书签显示的标题
const getBookmarkDisplayTitle = (bookmark: RawBookmark): string => {
  const override = findOverride(bookmark.url);
  if (override?.custom_title && override.custom_title.trim()) {
    return override.custom_title;
  }
  return bookmark.title;
};

// 切换书签排除状态
const toggleBookmarkExcluded = async (url: string, excluded: boolean) => {
  const overrides = [...getOverrides()];
  const existingIndex = overrides.findIndex(o => o.url === url);
  
  if (existingIndex >= 0) {
    if (!excluded && !overrides[existingIndex].custom_title) {
      // 如果取消排除且没有自定义标题，删除覆盖配置
      overrides.splice(existingIndex, 1);
    } else {
      overrides[existingIndex].excluded = excluded;
    }
  } else if (excluded) {
    // 新增覆盖配置
    overrides.push({
      url,
      excluded: true,
      custom_title: null,
    });
  }
  
  await updateOverrides(overrides);
};

// 更新覆盖配置
const updateOverrides = async (overrides: BookmarkOverride[]) => {
  remoteConfigStore.updateConfig({
    bookmark_loader_config: {
      overrides
    }
  });
};

// 加载书签列表
const loadBookmarks = async (path: string) => {
  loadingBookmarks.value = true;
  try {
    rawBookmarks.value = await invoke<RawBookmark[]>('read_browser_bookmarks', {
      bookmarksPath: path
    });
  } catch (e) {
    console.error('Failed to load bookmarks:', e);
    rawBookmarks.value = [];
  } finally {
    loadingBookmarks.value = false;
  }
};

// 打开编辑对话框
const openEditDialog = (bookmark: RawBookmark) => {
  editingBookmark.value = bookmark;
  const override = findOverride(bookmark.url);
  editingCustomTitle.value = override?.custom_title || '';
  editingExcluded.value = override?.excluded ?? false;
  editDialogVisible.value = true;
};

// 保存书签覆盖配置
const saveBookmarkOverride = async () => {
  if (!editingBookmark.value) return;
  
  const url = editingBookmark.value.url;
  const overrides = [...getOverrides()];
  const existingIndex = overrides.findIndex(o => o.url === url);
  
  const hasChanges = editingExcluded.value || editingCustomTitle.value.trim();
  
  if (existingIndex >= 0) {
    if (!hasChanges) {
      // 没有覆盖内容，删除配置
      overrides.splice(existingIndex, 1);
    } else {
      overrides[existingIndex] = {
        url,
        excluded: editingExcluded.value,
        custom_title: editingCustomTitle.value.trim() || null,
      };
    }
  } else if (hasChanges) {
    overrides.push({
      url,
      excluded: editingExcluded.value,
      custom_title: editingCustomTitle.value.trim() || null,
    });
  }
  
  await updateOverrides(overrides);
  editDialogVisible.value = false;
  ElMessage.success(t('settings.search.bookmarks.save_success'));
};

const loadConfig = async () => {
    const config = remoteConfigStore.config.bookmark_loader_config;
    if (config && config.sources) {
        bookmarkSources.value = [...config.sources];
    } else {
        bookmarkSources.value = [];
    }
};

const saveConfig = async () => {
    remoteConfigStore.updateConfig({
        bookmark_loader_config: {
            sources: bookmarkSources.value
        }
    });
};

const selectSource = async (index: number) => {
    selectedIndex.value = index;
    const source = bookmarkSources.value[index];
    if (source) {
      await loadBookmarks(source.bookmarks_path);
    }
};

// 当选中的书签源变化时，重新加载书签
watch(selectedIndex, async (newIndex) => {
  if (newIndex !== null && bookmarkSources.value[newIndex]) {
    await loadBookmarks(bookmarkSources.value[newIndex].bookmarks_path);
  } else {
    rawBookmarks.value = [];
  }
});

const detectAndAdd = async () => {
  detecting.value = true;
  try {
    const browsers = await invoke<BrowserInfo[]>('detect_installed_browsers');
    let addedCount = 0;
    
    for (const browser of browsers) {
      const exists = bookmarkSources.value.some(
        source => source.bookmarks_path === browser.bookmarks_path
      );
      
      if (!exists) {
        bookmarkSources.value.push({
          name: browser.name,
          bookmarks_path: browser.bookmarks_path,
          enabled: false,
        });
        addedCount++;
      }
    }
    
    if (addedCount > 0) {
      await saveConfig();
      ElMessage.success(t('settings.search.bookmarks.auto_detect') + `: +${addedCount}`);
    } else if (browsers.length === 0) {
      ElMessage.info(t('settings.search.bookmarks.no_browsers_found'));
    } else {
      ElMessage.info(t('settings.search.bookmarks.all_added'));
    }
  } catch (e) {
    console.error('Failed to detect browsers:', e);
    ElMessage.error('Failed to detect browsers');
  } finally {
    detecting.value = false;
  }
};

const removePath = async (index: number) => {
  if (selectedIndex.value === index) {
    selectedIndex.value = null;
    rawBookmarks.value = [];
  } else if (selectedIndex.value !== null && selectedIndex.value > index) {
    selectedIndex.value--;
  }
  bookmarkSources.value.splice(index, 1);
  await saveConfig();
};

onMounted(async () => {
    await loadConfig();
});
</script>

<style scoped>
.settings-page {
    padding: 20px;
    height: 100%;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
}

.path-config-container {
    display: flex;
    gap: 20px;
    flex: 1;
    min-height: 0;
    box-sizing: border-box;
}

.path-list-section {
    font-size: 14px;
    width: 250px;
    display: flex;
    flex-direction: column;
    border: 1px solid var(--el-border-color);
    border-radius: 4px;
    background-color: var(--el-bg-color);
}

.path-detail-section {
    flex: 1;
    padding: 15px;
    height: 100%;
    border: 1px solid var(--el-border-color);
    border-radius: 4px;
    overflow: hidden;
    box-sizing: border-box;
    background-color: var(--el-bg-color);
    display: flex;
    flex-direction: column;
}

.section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px;
    border-bottom: 1px solid var(--el-border-color);
    background-color: var(--el-fill-color-light);
}

.section-header h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
}

.section-header h4 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
}

.path-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px;
    border-bottom: 1px solid var(--el-border-color-lighter);
    cursor: pointer;
    transition: background-color 0.2s;
}

.path-item:hover {
    background-color: var(--el-fill-color-light);
}

.path-item.active {
    background-color: var(--el-color-primary-light-9);
    border-right: 2px solid var(--el-color-primary);
}

.path-item.disabled {
    opacity: 0.6;
}

.path-info {
    flex: 1;
    margin-right: 10px;
    overflow: hidden;
}

.path-name {
    font-size: 14px;
    font-weight: 500;
    display: flex;
    align-items: center;
    margin-bottom: 4px;
}

.path-name .el-icon {
    margin-right: 5px;
}

.path-text {
    font-size: 12px;
    color: var(--el-text-color-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.path-actions {
    display: flex;
    align-items: center;
    gap: 5px;
}

.add-path-btn {
    margin: 10px;
    margin-top: auto;
}

.detail-form {
    display: flex;
    flex-direction: column;
    gap: 15px;
    height: 100%;
    overflow: hidden;
}

.form-row {
    display: flex;
    align-items: center;
}

.form-label {
    min-width: 100px;
    font-size: 14px;
    color: var(--el-text-color-regular);
}

.empty-state {
    display: flex;
    justify-content: center;
    align-items: center;
    height: 100%;
}

/* Bookmark list styles */
.bookmark-list-section {
    border-top: 1px solid var(--el-border-color-light);
    padding-top: 10px;
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
}

.bookmark-item {
    display: flex;
    align-items: center;
    padding: 8px 10px;
    border-bottom: 1px solid var(--el-border-color-lighter);
    transition: all 0.2s;
}

.bookmark-item:hover {
    background-color: var(--el-fill-color-light);
}

.bookmark-item.excluded {
    opacity: 0.5;
    text-decoration: line-through;
}

.bookmark-info {
    flex: 1;
    overflow: hidden;
    margin-right: 10px;
}

.bookmark-title {
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: flex;
    align-items: center;
    gap: 6px;
}

.bookmark-url {
    font-size: 11px;
    color: var(--el-text-color-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.custom-badge {
    font-size: 10px;
    padding: 1px 4px;
    border-radius: 3px;
    background-color: var(--el-color-primary-light-8);
    color: var(--el-color-primary);
    text-decoration: none;
    display: inline-block;
}
</style>

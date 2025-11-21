<template>
    <div class="settings-page">
        <h1 class="page-title">{{ t('debug.title') }}</h1>

        <div class="content-container">
        <el-card class="performance-section">
            <template #header>
                <div class="card-header">
                    <h2>{{ t('debug.performance_test') }}</h2>
                </div>
            </template>

            <div class="performance-buttons">
                <el-button type="primary" @click="testSearchTime" :loading="searchTimeLoading">
                    {{ t('debug.test_search_time') }}
                </el-button>

                <el-button type="success" @click="testIndexTime" :loading="indexTimeLoading">
                    {{ t('debug.test_index_time') }}
                </el-button>
            </div>

            <div class="performance-results" v-if="searchTimeResult || indexTimeResult">
                <el-descriptions border direction="vertical">
                    <el-descriptions-item v-if="searchTimeResult" :label="t('debug.search_algorithm_time')">
                        {{ searchTimeResult }}
                    </el-descriptions-item>

                </el-descriptions>
                <el-descriptions border direction="vertical">
                    <el-descriptions-item v-if="indexTimeResult" :label="t('debug.index_file_time')">
                        {{ indexTimeResult }}
                    </el-descriptions-item>
                </el-descriptions>
            </div>
        </el-card>

        <el-card class="keyword-generator-section">
            <template #header>
                <div class="card-header">
                    <h2>{{ t('debug.keyword_generation') }}</h2>
                </div>
            </template>

            <el-input v-model="keywordInput" :placeholder="t('debug.input_parameter')" clearable
                @keyup.enter="generateSearchKeywords">
                <template #append>
                    <el-button @click="generateSearchKeywords" :loading="isGenerating">
                        {{ t('debug.do_generate_keywords') }}
                    </el-button>
                </template>
            </el-input>

            <div class="keyword-results" v-if="searchKeywords.length > 0">
                <h3>{{ t('debug.generated_keywords') }}</h3>
                <el-tag v-for="(keyword, index) in searchKeywords" :key="index" class="keyword-tag" closable
                    @close="removeKeyword(index)">
                    {{ keyword }}
                </el-tag>
            </div>
        </el-card>

        <el-card class="search-section">
            <template #header>
                <div class="card-header">
                    <h2>{{ t('debug.program_search') }}</h2>
                </div>
            </template>

            <el-input v-model="searchQuery" :placeholder="t('debug.input_search_keyword')" clearable
                @keyup.enter="handleSearch">
                <template #append>
                    <el-button @click="handleSearch" :loading="searchLoading">
                        {{ t('debug.search') }}
                    </el-button>
                </template>
            </el-input>

            <div class="result-table" v-if="searchResults.length > 0">
                <h3>{{ t('debug.search_results') }}</h3>
                <el-table :data="searchResults" stripe style="width: 100%" v-loading="searchLoading">
                    <el-table-column prop="program_name" :label="t('debug.program_name')" />
                    <el-table-column prop="program_keywords" :label="t('debug.keywords')" />
                    <el-table-column prop="program_path" :label="t('debug.program_path')" />
                    <el-table-column prop="score" :label="t('debug.weight_value')" />
                </el-table>
            </div>

            <div v-else-if="searchPerformed && !searchLoading" class="no-results">
                <el-empty :description="t('debug.no_matching_programs')" />
            </div>
        </el-card>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { ElMessage } from 'element-plus';
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

interface ProgramItem {
    program_name: string;
    program_keywords: string;
    program_path: string;
    score: number;
}

// 搜索相关
const searchQuery = ref('');
const searchResults = ref<ProgramItem[]>([]);
const searchLoading = ref(false);
const searchPerformed = ref(false);

// 性能测试相关
const searchTimeResult = ref('');
const indexTimeResult = ref('');
const searchTimeLoading = ref(false);
const indexTimeLoading = ref(false);

// 自定义函数相关
const keywordInput = ref('');
const searchKeywords = ref<string[]>([]);
const isGenerating = ref(false);

// 搜索程序
const handleSearch = async () => {
    if (!searchQuery.value.trim()) {
        ElMessage.warning(t('debug.please_input_search_keyword'));
        return;
    }

    searchLoading.value = true;
    searchPerformed.value = true;

    try {
        const results = await invoke<ProgramItem[]>('test_search_algorithm', { searchText: searchQuery.value });
        searchResults.value = results;
        if (results.length === 0) {
            ElMessage.info(t('debug.no_matching_programs'));
        }
    } catch (error) {
        console.error(t('debug.search_error'), error);
        ElMessage.error(t('debug.search_failed'));
        searchResults.value = [];
    } finally {
        searchLoading.value = false;
    }
};

// 测试搜索算法耗时
const testSearchTime = async () => {
    searchTimeLoading.value = true;
    try {
        const result = await invoke<[number, number, number]>('test_search_algorithm_time');
        // 完成格式的判断与输出
        const [maxTime, minTime, avgTime] = result;
        const threshold = 30; // 约33.33ms

        // 格式化时间，保留2位小数
        const formattedMax = maxTime.toFixed(2);
        const formattedMin = minTime.toFixed(2);
        const formattedAvg = avgTime.toFixed(2);

        let performanceStatus = '';
        if (maxTime < threshold) {
            performanceStatus = '✅ ' + t('debug.smooth');
        } else if (minTime > threshold) {
            performanceStatus = '❌ ' + t('debug.too_slow');
        } else {
            performanceStatus = '⚠️ ' + t('debug.partially_not_smooth');
        }

        searchTimeResult.value = `${t('debug.max_time')}: ${formattedMax}ms | ${t('debug.min_time')}: ${formattedMin}ms | ${t('debug.avg_time')}: ${formattedAvg}ms | ${performanceStatus}`;


        ElMessage.success(t('debug.search_test_completed'));
    } catch (error) {
        console.error(t('debug.test_algorithm_error'), error);
        ElMessage.error(t('debug.test_failed'));
    } finally {
        searchTimeLoading.value = false;
    }
};

const formatTime = (milliseconds: number): string => {
    if (milliseconds < 100) {
        return `${milliseconds.toFixed(2)}${t('debug.milliseconds')}`;
    } else if (milliseconds < 60000) {
        const seconds = milliseconds / 1000;
        return `${seconds.toFixed(2)}${t('debug.seconds')}`;
    } else {
        const minutes = Math.floor(milliseconds / 60000);
        const seconds = (milliseconds % 60000) / 1000;
        return `${minutes}${t('debug.minutes')} ${seconds.toFixed(2)}${t('debug.seconds')}`;
    }
};

// 测试索引文件耗时
const testIndexTime = async () => {
    indexTimeLoading.value = true;

    try {
        const result = await invoke<number>('test_index_app_time');
        indexTimeResult.value = formatTime(result);
        ElMessage.success(t('debug.index_test_completed'));
    } catch (error) {

        ElMessage.error(t('debug.test_failed'));
    } finally {
        indexTimeLoading.value = false;
    }
};

// 生成搜索关键字
const generateSearchKeywords = async () => {
    if (!keywordInput.value.trim()) {
        ElMessage.warning(t('debug.please_input_parameter'));
        return;
    }

    isGenerating.value = true;

    try {
        const keywords = await invoke<string[]>('get_search_keys', {
            showName: keywordInput.value
        });
        searchKeywords.value = keywords;
    } catch (error) {
        console.error(t('debug.keyword_generation_failed'), error);
        ElMessage.error(t('debug.keyword_generation_failed'));
    } finally {
        isGenerating.value = false;
    }
};

// 移除关键字
const removeKeyword = (index: number) => {
    searchKeywords.value.splice(index, 1);
};
</script>


<style scoped>
.settings-page {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 20px;
    box-sizing: border-box;
}

.page-title {
    margin-top: 0;
    margin-bottom: 20px;
    font-size: 24px;
    font-weight: 500;
    color: var(--el-text-color-primary);
    flex-shrink: 0;
}

.content-container {
    flex: 1;
    overflow-y: auto;
    padding-right: 10px;
}

.search-section,
.performance-section,
.keyword-generator-section {
    /* 添加了这个类 */
    margin-bottom: 30px;
}

.card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
}

h2 {
    margin: 0;
    font-size: 18px;
    color: #606266;
}

.result-table,
.performance-results,
.keyword-results {
    /* 修改了这个类名以匹配实际使用 */
    margin-top: 20px;
}

.no-results {
    margin-top: 20px;
    text-align: center;
}

.performance-buttons {
    display: flex;
    gap: 15px;
}

/* 添加关键字标签的样式 */
.keyword-tag {
    margin-right: 8px;
    margin-bottom: 8px;
}

@media (max-width: 768px) {
    .performance-buttons {
        flex-direction: column;
    }
}
</style>

<template>
    <div style="height: 100%; overflow-y: auto;">
        <h1>ZeroLaunch-rs 调试页面</h1>

        <el-card class="performance-section">
            <template #header>
                <div class="card-header">
                    <h2>性能测试</h2>
                </div>
            </template>

            <div class="performance-buttons">
                <el-button type="primary" @click="testSearchTime" :loading="searchTimeLoading">
                    测试搜索算法耗时
                </el-button>

                <el-button type="success" @click="testIndexTime" :loading="indexTimeLoading">
                    测试索引文件耗时
                </el-button>
            </div>

            <div class="performance-results" v-if="searchTimeResult || indexTimeResult">
                <el-descriptions border direction="vertical">
                    <el-descriptions-item v-if="searchTimeResult" label="搜索算法耗时">
                        {{ searchTimeResult }}
                    </el-descriptions-item>

                </el-descriptions>
                <el-descriptions border direction="vertical">
                    <el-descriptions-item v-if="indexTimeResult" label="索引文件耗时">
                        {{ indexTimeResult }}
                    </el-descriptions-item>
                </el-descriptions>
            </div>
        </el-card>

        <el-card class="keyword-generator-section">
            <template #header>
                <div class="card-header">
                    <h2>搜索关键字生成</h2>
                </div>
            </template>

            <el-input v-model="keywordInput" placeholder="输入参数" clearable @keyup.enter="generateSearchKeywords">
                <template #append>
                    <el-button @click="generateSearchKeywords" :loading="isGenerating">
                        生成搜索关键字
                    </el-button>
                </template>
            </el-input>

            <div class="keyword-results" v-if="searchKeywords.length > 0">
                <h3>生成的关键字（小写名字，拼音名字，首字母名字，所有大写字母组成的名字）</h3>
                <el-tag v-for="(keyword, index) in searchKeywords" :key="index" class="keyword-tag" closable
                    @close="removeKeyword(index)">
                    {{ keyword }}
                </el-tag>
            </div>
        </el-card>

        <el-card class="search-section">
            <template #header>
                <div class="card-header">
                    <h2>程序搜索</h2>
                </div>
            </template>

            <el-input v-model="searchQuery" placeholder="输入搜索关键词" clearable @keyup.enter="handleSearch">
                <template #append>
                    <el-button @click="handleSearch" :loading="searchLoading">
                        搜索
                    </el-button>
                </template>
            </el-input>

            <div class="result-table" v-if="searchResults.length > 0">
                <h3>搜索结果</h3>
                <el-table :data="searchResults" stripe style="width: 100%" v-loading="searchLoading">
                    <el-table-column prop="program_name" label="程序名称" />
                    <el-table-column prop="program_keywords" label="关键字" />
                    <el-table-column prop="program_path" label="程序路径" />
                    <el-table-column prop="score" label="权重值" />
                </el-table>
            </div>

            <div v-else-if="searchPerformed && !searchLoading" class="no-results">
                <el-empty description="没有找到匹配的程序" />
            </div>
        </el-card>
    </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { ElMessage } from 'element-plus';
import { invoke } from '@tauri-apps/api/core';

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
        ElMessage.warning('请输入搜索关键词');
        return;
    }

    searchLoading.value = true;
    searchPerformed.value = true;

    try {
        const results = await invoke<ProgramItem[]>('test_search_algorithm', { searchText: searchQuery.value });
        searchResults.value = results;
        if (results.length === 0) {
            ElMessage.info('没有找到匹配的程序');
        }
        console.log('已有结果')
    } catch (error) {
        console.error('搜索出错:', error);
        ElMessage.error('搜索失败，请查看控制台获取详细信息');
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
            performanceStatus = '✅ 流畅';
        } else if (minTime > threshold) {
            performanceStatus = '❌ 过慢';
        } else {
            performanceStatus = '⚠️ 部分情况可能不流畅';
        }

        searchTimeResult.value = `最大耗时: ${formattedMax}ms | 最小耗时: ${formattedMin}ms | 平均耗时: ${formattedAvg}ms | ${performanceStatus}`;


        ElMessage.success('搜索算法测试完成');
    } catch (error) {
        console.error('测试搜索算法出错:', error);
        ElMessage.error('测试失败，请查看控制台获取详细信息');
    } finally {
        searchTimeLoading.value = false;
    }
};

const formatTime = (milliseconds: number): string => {
    if (milliseconds < 100) {
        return `${milliseconds.toFixed(2)}毫秒`;
    } else if (milliseconds < 60000) {
        const seconds = milliseconds / 1000;
        return `${seconds.toFixed(2)}秒`;
    } else {
        const minutes = Math.floor(milliseconds / 60000);
        const seconds = (milliseconds % 60000) / 1000;
        return `${minutes}分 ${seconds.toFixed(2)}秒`;
    }
};

// 测试索引文件耗时
const testIndexTime = async () => {
    indexTimeLoading.value = true;

    try {
        const result = await invoke<number>('test_index_app_time');
        indexTimeResult.value = formatTime(result);
        ElMessage.success('索引文件测试完成');
    } catch (error) {
        console.error('测试索引文件出错:', error);
        ElMessage.error('测试失败，请查看控制台获取详细信息');
    } finally {
        indexTimeLoading.value = false;
    }
};

// 生成搜索关键字
const generateSearchKeywords = async () => {
    if (!keywordInput.value.trim()) {
        ElMessage.warning('请输入参数');
        return;
    }

    isGenerating.value = true;

    try {
        const keywords = await invoke<string[]>('get_search_keys', {
            showName: keywordInput.value
        });
        searchKeywords.value = keywords;
    } catch (error) {
        console.error('关键字生成失败:', error);
        ElMessage.error('生成关键字失败，请查看控制台获取详细信息');
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

h1 {
    text-align: center;
    margin-bottom: 30px;
    color: #409EFF;
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
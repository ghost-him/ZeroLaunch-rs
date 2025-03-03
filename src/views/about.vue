<template>
    <div class="about-container">
        <div class="about-header">
            <img src="../assets/icon.svg" alt="ZeroLaunch Logo" class="logo" />
            <h1>ZeroLaunch</h1>
            <div class="version-container">
                <span class="version">{{ currentVersion }}</span>
                <el-button type="primary" size="small" :loading="checkingUpdate" @click="checkUpdate"
                    class="update-btn">
                    检查更新
                </el-button>
            </div>
            <el-alert v-if="updateStatus" :title="updateStatus.message" :type=updateStatus.type />
        </div>

        <div class="about-content">
            <el-divider content-position="left">软件简介</el-divider>
            <p class="description">
                ZeroLaunch 是一款专为 Windows 平台精心打造的应用程序启动器，致力于提供极致高效、快捷的搜索体验，让您瞬间找到并启动所需应用。
            </p>

            <el-divider content-position="left">核心特性</el-divider>
            <div class="features">
                <div class="feature-item">
                    <el-icon>
                        <Location />
                    </el-icon>
                    <div class="feature-content">
                        <h3>本地运行</h3>
                        <p>完全离线运行，不会上传任何数据，保护您的隐私安全</p>
                    </div>
                </div>
                <div class="feature-item">
                    <el-icon>
                        <Search />
                    </el-icon>
                    <div class="feature-content">
                        <h3>智能搜索</h3>
                        <p>支持全称、拼音、模糊、首字母搜索，基于历史启动次数实现动态权重调节</p>
                    </div>
                </div>
                <div class="feature-item">
                    <el-icon>
                        <Aim />
                    </el-icon>
                    <div class="feature-content">
                        <h3>功能纯粹</h3>
                        <p>专注于应用程序搜索，支持自定义搜索路径与UWP应用搜索</p>
                    </div>
                </div>
                <div class="feature-item">
                    <el-icon>
                        <Share />
                    </el-icon>
                    <div class="feature-content">
                        <h3>开源项目</h3>
                        <p>采用GPLv3许可证，代码完全开源</p>
                    </div>
                </div>
            </div>

            <el-divider content-position="left">技术栈</el-divider>
            <div class="tech-stack">
                <el-tag>Rust</el-tag>
                <el-tag>Tauri</el-tag>
                <el-tag>Vue.js</el-tag>
                <el-tag>Element Plus</el-tag>
            </div>

            <el-divider content-position="left">项目地址</el-divider>
            <div class="repo-links">
                <el-link type="primary" href="https://github.com/ghost-him/ZeroLaunch-rs" target="_blank">
                    <el-icon class="link-icon">
                        <ElementPlus />
                    </el-icon>GitHub
                </el-link>
                <el-link type="success" href="https://gitee.com/ghost-him/ZeroLaunch-rs" target="_blank">
                    <el-icon class="link-icon">
                        <ElementPlus />
                    </el-icon>Gitee
                </el-link>
                <el-link type="warning" href="https://gitcode.com/ghost-him/ZeroLaunch-rs" target="_blank">
                    <el-icon class="link-icon">
                        <ElementPlus />
                    </el-icon>GitCode
                </el-link>
            </div>
        </div>

        <div class="about-footer">
            <p>© {{ new Date().getFullYear() }} ZeroLaunch - 基于 GPLv3 许可证开源</p>
        </div>
    </div>
</template>

<script setup>
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { Location, Search, Aim, Share, ElementPlus } from '@element-plus/icons-vue';


const currentVersion = ref('v0.4.0'); // 假设当前版本
const checkingUpdate = ref(false);
const updateStatus = ref(null);

// 检查更新函数
const checkUpdate = async () => {
    checkingUpdate.value = true;
    updateStatus.value = null;
    console.log('检查更新');
    try {

        const latestVersion = await invoke('command_get_latest_release_version');
        console.log(latestVersion);

        if (latestVersion === currentVersion.value) {
            updateStatus.value = {
                type: 'success',
                message: '您已经使用的是最新版本'
            };
        } else {
            updateStatus.value = {
                type: 'warning',
                message: `发现新版本: ${latestVersion}，请前往项目主页下载`
            };
        }
    } catch (error) {
        console.log(error)
        updateStatus.value = {
            type: 'error',
            message: '检查更新失败: ' + error
        };
    } finally {
        checkingUpdate.value = false; // 确保最终重置状态
    }
};

onMounted(async () => {
});
</script>

<style scoped>
.about-container {
    padding: 20px;
    height: 100%;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    color: #303133;
}

.about-header {
    display: flex;
    flex-direction: column;
    align-items: center;
    margin-bottom: 30px;
}

.logo {
    width: 80px;
    height: 80px;
    margin-bottom: 10px;
}

.about-header h1 {
    margin: 10px 0;
    font-size: 28px;
    font-weight: 600;
}

.version-container {
    display: flex;
    align-items: center;
    margin: 10px 0;
}

.version {
    margin-right: 10px;
    font-size: 14px;
    color: #909399;
}

.update-btn {
    font-size: 12px;
}

.update-status {
    margin-top: 10px;
    padding: 8px 16px;
    border-radius: 4px;
    font-size: 14px;
}

.update-status.success {
    background-color: #f0f9eb;
    color: #67c23a;
}

.update-status.warning {
    background-color: #fdf6ec;
    color: #e6a23c;
}

.update-status.error {
    background-color: #fef0f0;
    color: #f56c6c;
}

.about-content {
    flex: 1;
}

.description {
    font-size: 16px;
    line-height: 1.6;
    margin: 20px 0;
    text-align: justify;
}

.features {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 20px;
    margin: 20px 0;
}

.feature-item {
    display: flex;
    align-items: flex-start;
    padding: 15px;
    border-radius: 8px;
    background-color: #f5f7fa;
    transition: transform 0.3s, box-shadow 0.3s;
}

.feature-item:hover {
    transform: translateY(-5px);
    box-shadow: 0 5px 15px rgba(0, 0, 0, 0.1);
}

.feature-item .el-icon {
    font-size: 24px;
    color: #409eff;
    margin-right: 15px;
    margin-top: 5px;
}

.feature-content h3 {
    margin: 0 0 8px 0;
    font-size: 16px;
    font-weight: 600;
}

.feature-content p {
    margin: 0;
    font-size: 14px;
    color: #606266;
    line-height: 1.5;
}

.tech-stack {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin: 20px 0;
}

.tech-stack .el-tag {
    padding: 6px 12px;
    font-size: 14px;
}

.repo-links {
    display: flex;
    flex-wrap: wrap;
    gap: 20px;
    margin: 20px 0;
}

.repo-links .el-link {
    display: flex;
    align-items: center;
    font-size: 16px;
}

.link-icon {
    margin-right: 5px;
}

.author-info {
    margin: 20px 0;
    line-height: 1.6;
}

.about-footer {
    margin-top: 30px;
    text-align: center;
    color: #909399;
    font-size: 14px;
}

:deep(.el-divider__text) {
    font-size: 18px;
    font-weight: 600;
    color: #409eff;
}
</style>
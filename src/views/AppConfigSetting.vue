<template>
    <el-tabs style="height: 100%;">
        <el-tab-pane label="通用设置" style="height: 100%;overflow-y: auto;">
            <el-form label-width="auto">
                <el-divider content-position="left">启动与实例</el-divider>
                <el-form-item label="设置开机自启动">
                    <el-switch v-model="config.app_config.is_auto_start"
                        @change="(val: boolean) => configStore.updateConfig({ app_config: { is_auto_start: val } })" />
                </el-form-item>

                <el-form-item label="设置静默启动">
                    <el-switch v-model="config.app_config.is_silent_start"
                        @change="(val: boolean) => configStore.updateConfig({ app_config: { is_silent_start: val } })" />
                </el-form-item>

                <el-form-item label="当唤醒程序失败时启动新实例">
                    <el-switch v-model="config.app_config.launch_new_on_failure"
                        @change="(val: boolean) => configStore.updateConfig({ app_config: { launch_new_on_failure: val } })" />
                </el-form-item>

                <el-divider content-position="left">搜索与数据</el-divider>
                <el-form-item label="设置搜索结果数量">
                    <el-input-number v-model="config.app_config.search_result_count" :step="1" :precision="0" :min="1"
                        @change="(val: number) => configStore.updateConfig({ app_config: { search_result_count: val } })" />
                </el-form-item>

                <el-form-item label="自动刷新数据库的时间">
                    <el-input-number v-model="config.app_config.auto_refresh_time" :step="1" :precision="0" :min="1"
                        @change="(val: number) => configStore.updateConfig({ app_config: { auto_refresh_time: val } })">
                        <template #suffix>
                            <span>分钟</span>
                        </template>
                    </el-input-number>
                </el-form-item>

                <el-divider content-position="left">窗口与交互</el-divider>
                <el-form-item label="esc键优先关闭窗口">
                    <el-switch v-model="config.app_config.is_esc_hide_window_priority"
                        @change="(val: boolean) => configStore.updateConfig({ app_config: { is_esc_hide_window_priority: val } })" />
                    <el-tooltip class="box-item" effect="dark" content="默认优先清空搜索栏后关闭">
                        <el-icon class="el-question-icon">
                            <QuestionFilled />
                        </el-icon>
                    </el-tooltip>
                </el-form-item>

                <el-form-item label="启用拖动窗口">
                    <el-switch v-model="config.app_config.is_enable_drag_window"
                        @change="(val: boolean) => configStore.updateConfig({ app_config: { is_enable_drag_window: val } })" />
                    <el-tooltip class="box-item" effect="dark" content="程序在下一次打开时会记忆上次的关闭的位置">
                        <el-icon class="el-question-icon">
                            <QuestionFilled />
                        </el-icon>
                    </el-tooltip>
                </el-form-item>

                <el-form-item label="全屏时是否可以唤醒窗口">
                    <el-switch v-model="config.app_config.is_wake_on_fullscreen"
                        @change="(val: boolean) => configStore.updateConfig({ app_config: { is_wake_on_fullscreen: val } })" />
                    <el-tooltip class="box-item" effect="dark"
                        content="与游戏模式的区别：游戏模式会取消注册唤醒的快捷键，让游戏可以接收到这个快捷键在，而该选项依然会保留快捷键">
                        <el-icon class="el-question-icon">
                            <QuestionFilled />
                        </el-icon>
                    </el-tooltip>
                </el-form-item>

                <el-form-item label="按下空格启动程序">
                    <el-switch v-model="config.app_config.space_is_enter"
                        @change="(val: boolean) => configStore.updateConfig({ app_config: { space_is_enter: val } })" />
                    <el-tooltip class="box-item" effect="dark" content="启动后，空格键的作用与enter键一样，同时无法再输入空格键">
                        <el-icon class="el-question-icon">
                            <QuestionFilled />
                        </el-icon>
                    </el-tooltip>
                </el-form-item>

                <el-divider content-position="left">高级</el-divider>
                <el-form-item label="调试模式">
                    <el-switch v-model="config.app_config.is_debug_mode"
                        @change="(val: boolean) => configStore.updateConfig({ app_config: { is_debug_mode: val } })" />
                </el-form-item>
            </el-form>
        </el-tab-pane>

        <!-- 这里可以根据需要添加更多的 el-tab-pane 来组织其他设置 -->

    </el-tabs>
</template>

<script lang="ts" setup>

import { QuestionFilled } from '@element-plus/icons-vue'
import { useRemoteConfigStore } from '../stores/remote_config'; // 确认路径正确
import { storeToRefs } from 'pinia';

const configStore = useRemoteConfigStore()
const { config } = storeToRefs(configStore)

</script>

<style>
/* 确保你的样式（例如 .el-question-icon）仍然可用 */
.el-question-icon {
    margin-left: 8px;
}

.el-icon {
    font-size: 18px;
    color: #606266;
}
</style>
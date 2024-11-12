<template>
    <el-tabs v-model="activeName" class="demo-tabs" @tab-click="handleClick">
        <el-tab-pane label="程序设置" name="first">
            <el-form :model="config" label-width="auto" style="max-width: 600px">
                
                <el-form-item label="自定义搜索栏的提示文本">
                <el-input v-model="config.search_bar_placeholder" placeholder="Hello, ZeroLaunch!" />
                </el-form-item>

                <el-form-item label="自定义搜索无结果的文本">
                <el-input v-model="config.search_bar_no_result" placeholder="当前搜索无结果" />
                </el-form-item>
                
                <el-form-item label="设置开机自启动">
                <el-switch v-model="config.is_auto_start" />
                </el-form-item>

                <el-form-item label="设置资源预加载">
                <el-switch v-model="config.is_preload_resource" />
                </el-form-item>

                <el-form-item label="设置搜索结果数量">
                <el-input-number v-model="config.search_result_count" step="1"/>
                </el-form-item>

                <el-form-item label="自动刷新数据库的时间（分钟）">
                <el-input-number v-model="config.auto_refresh_time" step="1"/>
                </el-form-item>
                <el-button type="primary" @click="onSubmit">提交</el-button>
            </el-form>
        </el-tab-pane>
        <el-tab-pane label="自定义搜索路径" name="second">Config</el-tab-pane>
        <el-tab-pane label="关键字过滤设置" name="third">Role</el-tab-pane>
        <el-tab-pane label="查看当前索引的所有的程序" name="fourth">Task</el-tab-pane>
    </el-tabs>
</template>
<script lang="ts" setup>
import { ref, reactive, onMounted, onUnmounted } from 'vue'
import type { TabsPaneContext } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'

const activeName = ref('first')
const handleClick = (tab: TabsPaneContext, event: Event) => {
    console.log(tab, event)
}

// do not use same name with ref
const config = reactive({
    search_bar_placeholder: '',
    search_bar_no_result: '',
    is_auto_start: false,
    is_preload_resource: false,
    search_result_count: 4,
    auto_refresh_time: 30,
})

const onSubmit = () => {
    save_app_config()
}

const get_app_config = async () => {
    const loadedConfig = await invoke('get_app_config')
    Object.assign(config, loadedConfig)
}

const save_app_config = async () => {
    await invoke('save_app_config', {appConfig: config})
}

onMounted(async () => {
    await get_app_config();

});

onUnmounted(() => {
    
})


</script>

<style>

</style>
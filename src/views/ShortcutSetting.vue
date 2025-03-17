<template>
    <div class="shortcut-settings-container">
        <el-form-item class="shortcut-form-item">
            <ShortcutInput label="唤醒窗口" v-model="shortcut" @before-change="handleUnbindShortcut"
                :defaultValue="default_shortcut" @after-change="handleBindShortcut"></ShortcutInput>
            <el-button class="reset-button" @click="reset_shortcut">
                <i class="el-icon-refresh-right"></i>
                重置
            </el-button>
        </el-form-item>
    </div>
</template>


<script lang="ts" setup>
import { Shortcut } from '../api/remote_config_types';
import ShortcutInput from '../utils/ShortcutInput.vue';
import { ref } from 'vue';
import { onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useRemoteConfigStore } from '../stores/remote_config';
import { ElMessage } from 'element-plus';
const default_shortcut = {
    key: 'Space',
    alt: true,
    ctrl: false,
    shift: false,
    meta: false
} as Shortcut;

interface ShortcutUnit {
    id: string,
    shortcut: Shortcut,
}

const configStore = useRemoteConfigStore()

const shortcut = ref(default_shortcut as Shortcut);
const id = ref('');
const handleUnbindShortcut = async () => {
    // 直接特殊写法,后期如果多了,这里可以传入目标的id,这样可以实现指定的快捷键的更换
    try {
        await invoke('delete_shortcut', { id: id.value });
    } catch (error) {
        handleError("快捷键解绑失败:" + error);
        // 可以添加用户提示
        // showErrorMessage("快捷键解绑失败，请重试");
    }
}

const handleBindShortcut = async () => {
    console.log("调用了一次");
    if (!shortcut.value || !id.value) {
        handleError("快捷键或ID不能为空");
        // showErrorMessage("快捷键不能为空");
        return;
    }
    try {
        await invoke('register_shortcut', { id: id.value, shortcut: shortcut.value });
        ElMessage.success("快捷键绑定成功");
        configStore.updateConfig({ app_config: { shortcut: shortcut.value } });
        configStore.syncConfig();
    } catch (error) {
        handleError("快捷键绑定失败: " + error + "已恢复默认配置");

        shortcut.value = default_shortcut;
        await invoke('register_shortcut', { id: id.value, shortcut: shortcut.value });
        // showErrorMessage("快捷键绑定失败，可能与系统其他快捷键冲突");
    }
}

const reset_shortcut = async () => {
    await handleUnbindShortcut()

    shortcut.value = default_shortcut;

    await handleBindShortcut()
}

const handleError = (error: string) => {
    ElMessage({
        showClose: true,
        message: error,
        type: 'error',
    })
}


onMounted(async () => {
    try {
        const data = await invoke<ShortcutUnit[]>('get_all_shortcut');
        // 目前只有一个快捷键可自定义,所以这里直接特殊写
        if (data && data.length > 0) {
            id.value = data[0].id;
            shortcut.value = data[0].shortcut;
        } else {
            console.warn("没有找到可用的快捷键配置");
        }
    } catch (error) {
        handleError("获取快捷键配置失败:" + error);
        // showErrorMessage("获取快捷键配置失败，请重启应用");
    }
})


</script>



<style scoped>
.shortcut-form-item {
    margin-bottom: 24px;
    background-color: #ffffff;
    padding: 20px;
    border-radius: 8px;
    box-shadow: 0 2px 12px 0 rgba(0, 0, 0, 0.05);
}

.reset-button {
    margin-left: 12px;
    font-size: 13px;
    color: #606266;
    transition: color 0.2s;
    padding: 7px 12px;
    border-radius: 4px;
}

.reset-button:hover {
    color: #409eff;
    background-color: #f0f7ff;
}

:deep(.el-form-item__content) {
    display: flex;
    align-items: center;
}
</style>
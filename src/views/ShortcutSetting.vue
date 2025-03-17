<template>
    <el-form-item>
        <ShortcutInput label="唤醒窗口" v-model="shortcut" @before-change="handleUnbindShortcut"
            :defaultValue="default_shortcut" @after-change="handleBindShortcut"></ShortcutInput>

    </el-form-item>

</template>


<script lang="ts" setup>
import { Shortcut } from '../api/remote_config_types';
import ShortcutInput from '../utils/ShortcutInput.vue';
import { ref } from 'vue';
import { onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useRemoteConfigStore } from '../stores/remote_config';
import { storeToRefs } from 'pinia';
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
    console.log("按下了");
    try {
        await invoke('delete_shortcut', { id: id.value });
        console.log("快捷键解绑成功");
    } catch (error) {
        console.error("快捷键解绑失败:", error);
        // 可以添加用户提示
        // showErrorMessage("快捷键解绑失败，请重试");
    }
}

const handleBindShortcut = async () => {
    console.log(shortcut);
    console.log("结束");

    if (!shortcut.value || !id.value) {
        console.error("快捷键或ID不能为空");
        // showErrorMessage("快捷键不能为空");
        return;
    }

    try {
        await invoke('register_shortcut', { id: id.value, shortcut: shortcut.value });
        console.log("快捷键绑定成功");
        // configStore.updateConfig({ app_config: { shortcut: shortcut.value } });
    } catch (error) {
        console.error("快捷键绑定失败:", error);
        // showErrorMessage("快捷键绑定失败，可能与系统其他快捷键冲突");
    }
}

onMounted(async () => {
    try {
        console.log("开始获取信息");
        const data = await invoke<ShortcutUnit[]>('get_all_shortcut');
        console.log(data);
        // 目前只有一个快捷键可自定义,所以这里直接特殊写
        if (data && data.length > 0) {
            id.value = data[0].id;
            shortcut.value = data[0].shortcut;
        } else {
            console.warn("没有找到可用的快捷键配置");
        }
    } catch (error) {
        console.error("获取快捷键配置失败:", error);
        // showErrorMessage("获取快捷键配置失败，请重启应用");
    }
})


</script>
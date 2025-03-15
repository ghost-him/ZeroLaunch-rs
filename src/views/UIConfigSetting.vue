<template>
    <el-tabs style="height: 100%">
        <el-tab-pane label="搜索栏与结果栏设置">
            <el-form-item label="搜索栏与状态栏的背景颜色">
                <el-color-picker v-model="config.ui_config.search_bar_background_color" show-alpha
                    @change="(val: string) => configStore.updateConfig({ ui_config: { search_bar_background_color: rgbaToHex(val) } })" />
            </el-form-item>
            <el-form-item label="设置结果栏的背景颜色">
                <el-color-picker v-model="config.ui_config.selected_item_color" show-alpha
                    @change="(val: string) => configStore.updateConfig({ ui_config: { selected_item_color: rgbaToHex(val) } })" />
            </el-form-item>
            <el-form-item label="搜索栏字体的颜色">
                <el-color-picker v-model="config.ui_config.search_bar_font_color"
                    @change="(val: string) => configStore.updateConfig({ ui_config: { search_bar_font_color: rgbaToHex(val) } })" />
            </el-form-item>
            <el-form-item label="设置结果栏的字体颜色">
                <el-color-picker v-model="config.ui_config.item_font_color"
                    @change="(val: string) => configStore.updateConfig({ ui_config: { item_font_color: rgbaToHex(val) } })" />
            </el-form-item>
            <el-form-item label="搜索栏的字体大小">
                <el-input-number v-model="config.ui_config.search_bar_font_size" placeholder="2" :min="0" :step="0.1"
                    @change="(val: number) => configStore.updateConfig({ ui_config: { search_bar_font_size: val } })">
                    <template #suffix>
                        <span>rem</span>
                    </template>
                </el-input-number>
                <el-tooltip class="box-item" effect="dark" placement="right-start"
                    content="单位rem：1rem表示1倍字体的高度，1.3rem表示1.3倍字体的高度">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>
            <el-form-item label="结果栏的字体大小">
                <el-input-number v-model="config.ui_config.item_font_size" placeholder="1.3" :min="0" :step="0.1"
                    @change="(val: number) => configStore.updateConfig({ ui_config: { item_font_size: val } })">
                    <template #suffix>
                        <span>rem</span>
                    </template>
                </el-input-number>
                <el-tooltip class="box-item" effect="dark" placement="right-start"
                    content="单位rem：1rem表示1倍字体的高度，1.3rem表示1.3倍字体的高度">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>
        </el-tab-pane>

        <el-tab-pane label="背景图片设置">
            <el-form-item label="毛玻璃效果">
                <el-select v-model="blur_style_value" placeholder="Select" style="width: 240px">
                    <el-option v-for="item in blur_style_option" :key="item.value" :label="item.label"
                        :disabled="item.disabled" :value="item.value" />
                </el-select>
                <el-tooltip class="box-item" effect="dark" placement="right-start"
                    content="目前tauri框架有bug，无法实现毛玻璃效果，需要等待更新">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>

            <el-form-item label="背景图片的大小">
                <el-input v-model="config.ui_config.background_size" style="max-width: 120px;" placeholder="cover"
                    @change="(val: string) => configStore.updateConfig({ ui_config: { background_size: val } })" />
                <el-tooltip class="box-item" effect="dark" placement="right-start"
                    content="'cover': 缩放图片以完全覆盖元素区域，保持比例；'contain:' 缩放图片以完全显示在元素内，保持比例；'auto':使用图片原始尺寸">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>

            <el-form-item label="背景图片的位置">
                <el-input v-model="config.ui_config.background_position" style="max-width: 120px;" placeholder="center"
                    @change="(val: string) => configStore.updateConfig({ ui_config: { background_position: val } })" />
                <el-tooltip class="box-item" effect="dark" placement="right-start"
                    content="图片的对齐位置，可选：'center', 'top', 'right', 'bottom', 'left'及其结合，例如: 'right bottom'">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>

            </el-form-item>

            <el-form-item label="背景图片是否重复显示">
                <el-input v-model="config.ui_config.background_repeat" style="max-width: 120px;" placeholder="no-repeat"
                    @change="(val: string) => configStore.updateConfig({ ui_config: { background_repeat: val } })" />
                <el-tooltip class="box-item" effect="dark" placement="right-start"
                    content="'no-repeat': 不重复；'repeat': 水平和垂直方向都重复；'repeat-x': 仅水平方向重复；'repeat-y': 仅垂直方向重复">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>

            <el-form-item label="背景图片透明度">
                <el-input-number v-model="config.ui_config.background_opacity" placeholder="65" :min="0" :max="1"
                    :step="0.1"
                    @change="(val: number) => configStore.updateConfig({ ui_config: { background_opacity: val } })" />
                <el-tooltip class="box-item" effect="dark" placement="right-start"
                    content="取值范围：[0, 1]。1表示完全不透明，0表示完全透明">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>

            <el-form-item label="选择背景图片">
                <el-button type="primary" @click="select_background_picture">选择图片</el-button>
                <el-button type="danger" @click="delete_background_picture">删除图片</el-button>

            </el-form-item>
            <el-form-item label="计算一个图片的主题色">
                <el-button type="primary" @click="get_dominant_color">选择图片</el-button>
                <el-tooltip class="box-item" effect="dark" placement="right-start"
                    content="如果不会选择结果栏的选中项颜色，可以使用该功能计算背景图片的主题色">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
                <div v-if="dominant_color"> 该图片的主题色为: {{ dominant_color }} </div>
            </el-form-item>
        </el-tab-pane>

        <el-tab-pane label="程序窗口设置">
            <el-form-item label="窗口垂直方向偏移比例因子">
                <el-input-number v-model="config.ui_config.vertical_position_ratio" placeholder="0.4" :min="0"
                    :step="0.05" :max="1"
                    @change="(val: number) => configStore.updateConfig({ ui_config: { vertical_position_ratio: val } })" />
                <el-tooltip class="box-item" effect="dark" placement="right-start"
                    content="0表示在屏幕顶部，1表示在屏幕底部，0.5表示在屏幕正中间">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>

            <el-form-item label="搜索栏的高度">
                <el-input-number v-model="config.ui_config.search_bar_height" placeholder="65" :min="1" :step="1"
                    :precision="0"
                    @change="(val: number) => configStore.updateConfig({ ui_config: { search_bar_height: val } })">
                    <template #suffix>
                        <span>px</span>
                    </template>
                </el-input-number>
                <el-tooltip class="box-item" effect="dark" placement="right-start" content="单位px：是数字图像的最小单位，是屏幕显示的基本点">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>

            <el-form-item label="结果栏中一项的高度">
                <el-input-number v-model="config.ui_config.result_item_height" placeholder="62" :min="1" :step="1"
                    :precision="0"
                    @change="(val: number) => configStore.updateConfig({ ui_config: { result_item_height: val } })">
                    <template #suffix>
                        <span>px</span>
                    </template>
                </el-input-number>
            </el-form-item>

            <el-form-item label="底栏的高度">
                <el-input-number v-model="config.ui_config.footer_height" placeholder="42" :min="0" :step="1"
                    :precision="0"
                    @change="(val: number) => configStore.updateConfig({ ui_config: { footer_height: val } })">
                    <template #suffix>
                        <span>px</span>
                    </template>
                </el-input-number>
            </el-form-item>

            <el-form-item label="程序的宽度">
                <el-input-number v-model="config.ui_config.window_width" placeholder="1000" :min="400" :step="1"
                    :precision="0"
                    @change="(val: number) => configStore.updateConfig({ ui_config: { window_width: val } })">
                    <template #suffix>
                        <span>px</span>
                    </template>
                </el-input-number>
            </el-form-item>
        </el-tab-pane>

    </el-tabs>
</template>

<script lang="ts" setup>
import { rgbaToHex } from '../utils/color';
import { QuestionFilled } from '@element-plus/icons-vue'
import { invoke } from '@tauri-apps/api/core';
import { ElMessage } from 'element-plus';
import { open } from '@tauri-apps/plugin-dialog';
import { ref, watch } from 'vue';
import { useRemoteConfigStore } from '../stores/remote_config';
import { storeToRefs } from 'pinia';
import { BlurStyle } from '../api/remote_config_types';

const configStore = useRemoteConfigStore()
const { config } = storeToRefs(configStore)
const blur_style_value = ref('None')

watch(blur_style_value, (newValue) => {
    const value = newValue as BlurStyle
    configStore.updateConfig({ ui_config: { blur_style: value } })
})

const blur_style_option = [
    {
        value: 'None',
        label: '无效果',
        disabled: true,
    },
    // {
    //     value: 'Blur',
    //     label: '毛玻璃效果',
    //     disabled: true,
    // },
    // {
    //     value: 'Acrylic',
    //     label: '亚克力效果',
    //     disabled: true,
    // },
    // {
    //     value: 'Mica',
    //     label: '云母效果',
    //     disabled: true,
    // },
]



const select_background_picture = async () => {
    let file_path = await select_picture();
    if (file_path) {
        console.log(file_path)
        invoke("select_background_picture", { path: file_path });
        ElMessage({
            message: '图片已保存',
            type: 'success',
        })
    }
}

const delete_background_picture = () => {
    invoke("select_background_picture", { path: "" });
    ElMessage({
        message: '图片已删除',
        type: 'success',
    })
}

const select_picture = async () => {
    const file_path = await open({
        canCreateDirectories: false,  // 禁止创建目录
        directory: false,             // 禁止选择目录
        multiple: false,              // 只允许选择一个文件
        title: "选择一个图片",         // 文件选择框的标题
        filters: [
            {
                name: 'Images',  // 过滤器的名称
                extensions: ['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp']  // 允许的图片文件扩展名
            }
        ]
    });
    return file_path;
}

let dominant_color = ref<string | null>(null);

const get_dominant_color = async () => {
    let file_path = await select_picture();
    let ret = await invoke<string>('get_dominant_color', { path: file_path });
    dominant_color.value = ret;
}


</script>

<style>
.el-question-icon {
    margin-left: 8px;
}

.el-icon {
    font-size: 18px;
    color: #606266;
}
</style>
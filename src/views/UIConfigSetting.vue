<template>
    <el-tabs style="height: 100%;">
        <el-tab-pane label="搜索栏与结果栏设置" style="height: 100%;overflow-y: auto;">
            <el-divider content-position="left">提示词</el-divider>
            <el-form-item label="自定义搜索栏的提示文本">
                <el-input v-model="config.app_config.search_bar_placeholder" placeholder="Hello, ZeroLaunch!"
                    @change="(val: string) => configStore.updateConfig({ app_config: { search_bar_placeholder: val } })" />
            </el-form-item>

            <el-form-item label="自定义底部提示栏">
                <el-input v-model="config.app_config.tips" placeholder="ZeroLaunch-rs v0.4.0"
                    @change="(val: string) => configStore.updateConfig({ app_config: { tips: val } })" />
            </el-form-item>
            <el-divider content-position="left">背景色</el-divider>
            <el-form-item label="整体的背景色（搜索栏与结果栏的整体的颜色）">
                <el-color-picker v-model="config.ui_config.program_background_color" show-alpha
                    @change="(val: string) => configStore.updateConfig({ ui_config: { program_background_color: rgbaToHex(val) } })" />
            </el-form-item>

            <el-form-item label="搜索栏与状态栏的背景颜色">
                <el-color-picker v-model="config.ui_config.search_bar_background_color" show-alpha
                    @change="(val: string) => configStore.updateConfig({ ui_config: { search_bar_background_color: rgbaToHex(val) } })" />
            </el-form-item>

            <el-form-item label="设置结果栏的选中项高亮颜色">
                <el-color-picker v-model="config.ui_config.selected_item_color" show-alpha
                    @change="(val: string) => configStore.updateConfig({ ui_config: { selected_item_color: rgbaToHex(val) } })" />
            </el-form-item>

            <el-form-item label="深色模式推荐配色">
                <el-tooltip class="box-item" effect="dark"
                    content="整体的背景色：rgba(31, 31, 31, 1),选中项高亮色：rgba(63, 63, 63, 0.8)，字体颜色：#A6A6A6">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>

            <el-divider content-position="left">搜索栏</el-divider>
            <el-form-item label="搜索栏字体设置">
                <el-select v-model="config.ui_config.search_bar_font_family" filterable placeholder="选择或输入字体"
                    @change="(val: string) => configStore.updateConfig({ ui_config: { search_bar_font_family: val } })">
                    <el-option v-for="font in systemFonts" :key="font" :label="font" :value="font">
                        <span :style="{ fontFamily: font }">{{ font }}</span>
                    </el-option>
                </el-select>
            </el-form-item>
            <el-form-item label="搜索栏字体的颜色">
                <el-color-picker v-model="config.ui_config.search_bar_font_color"
                    @change="(val: string) => configStore.updateConfig({ ui_config: { search_bar_font_color: rgbaToHex(val) } })" />
            </el-form-item>
            <el-form-item label="搜索栏的提示字体的颜色">
                <el-color-picker v-model="config.ui_config.search_bar_placeholder_font_color"
                    @change="(val: string) => configStore.updateConfig({ ui_config: { search_bar_placeholder_font_color: rgbaToHex(val) } })" />
            </el-form-item>
            <el-form-item label="搜索栏的字体大小(与行高占比大小)">
                <el-input-number v-model="config.ui_config.search_bar_font_size" placeholder="50" :min="5" :step="5"
                    :max="100"
                    @change="(val: number) => configStore.updateConfig({ ui_config: { search_bar_font_size: val } })">
                    <template #suffix>
                        <span>%</span>
                    </template>
                </el-input-number>
                <el-tooltip class="box-item" effect="dark" content="取值：[5, 100]，单位%：80%表示字体的高度为搜索栏高度的80%">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>
            <el-divider content-position="left">结果栏</el-divider>
            <el-form-item label="结果栏字体设置">
                <el-select v-model="config.ui_config.result_item_font_family" filterable placeholder="选择或输入字体"
                    @change="(val: string) => configStore.updateConfig({ ui_config: { result_item_font_family: val } })">
                    <el-option v-for="font in systemFonts" :key="font" :label="font" :value="font">
                        <span :style="{ fontFamily: font }">{{ font }}</span>
                    </el-option>
                </el-select>
            </el-form-item>
            <el-form-item label="设置结果栏的字体颜色">
                <el-color-picker v-model="config.ui_config.item_font_color"
                    @change="(val: string) => configStore.updateConfig({ ui_config: { item_font_color: rgbaToHex(val) } })" />
            </el-form-item>
            <el-form-item label="结果栏的字体大小(与行高占比大小)">
                <el-input-number v-model="config.ui_config.item_font_size" placeholder="33" :min="5" :step="5"
                    :max="100"
                    @change="(val: number) => configStore.updateConfig({ ui_config: { item_font_size: val } })">
                    <template #suffix>
                        <span>%</span>
                    </template>
                </el-input-number>
                <el-tooltip class="box-item" effect="dark" content="取值：[5, 100]，单位%：80%表示字体的高度为结果栏一项高度的80%">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>
            <el-divider content-position="left">底栏</el-divider>
            <el-form-item label="底栏字体设置">
                <el-select v-model="config.ui_config.footer_font_family" filterable placeholder="选择或输入字体"
                    @change="(val: string) => configStore.updateConfig({ ui_config: { footer_font_family: val } })">
                    <el-option v-for="font in systemFonts" :key="font" :label="font" :value="font">
                        <span :style="{ fontFamily: font }">{{ font }}</span>
                    </el-option>
                </el-select>
            </el-form-item>
            <el-form-item label="设置底栏的字体颜色">
                <el-color-picker v-model="config.ui_config.footer_font_color"
                    @change="(val: string) => configStore.updateConfig({ ui_config: { footer_font_color: rgbaToHex(val) } })" />
            </el-form-item>
            <el-form-item label="底栏的字体大小(与行高占比大小)">
                <el-input-number v-model="config.ui_config.footer_font_size" placeholder="33" :min="5" :step="5"
                    :max="100"
                    @change="(val: number) => configStore.updateConfig({ ui_config: { footer_font_size: val } })">
                    <template #suffix>
                        <span>%</span>
                    </template>
                </el-input-number>
                <el-tooltip class="box-item" effect="dark" content="取值：[5, 100]，单位%：80%表示字体的高度为底栏高度的80%">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>
        </el-tab-pane>

        <el-tab-pane label="背景图片设置" style="height: 100%;overflow-y: auto;">
            <el-form-item label="毛玻璃效果">
                <el-select v-model="config.ui_config.blur_style" placeholder="Select" style="width: 240px"
                    :disabled="!config.ui_config.use_windows_sys_control_radius"
                    @change="(val: string) => configStore.updateConfig({ ui_config: { blur_style: val } })">
                    <el-option v-for="item in blur_style_option" :key="item.value" :label="item.label"
                        :value="item.value" />
                </el-select>
                <el-tooltip class="box-item" effect="dark" content="仅支持使用系统圆角">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>

            <el-form-item label="背景图片的大小">
                <el-select v-model="config.ui_config.background_size" placeholder="cover" style="width: 240px"
                    @change="(val: string) => configStore.updateConfig({ ui_config: { background_size: val } })">
                    <el-option v-for="item in background_size" :key="item.value" :label="item.label"
                        :value="item.value" />
                </el-select>
                <el-tooltip class="box-item" effect="dark"
                    content="'cover': 缩放图片以完全覆盖元素区域，保持比例；'contain:' 缩放图片以完全显示在元素内，保持比例；'auto':使用图片原始尺寸">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>

            <el-form-item label="背景图片的位置">
                <el-select v-model="config.ui_config.background_position" placeholder="center" style="width: 240px"
                    @change="(val: string) => configStore.updateConfig({ ui_config: { background_position: val } })">
                    <el-option v-for="item in background_position" :key="item.value" :label="item.label"
                        :value="item.value" />
                </el-select>
                <el-tooltip class="box-item" effect="dark" content="图片的对齐位置">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>

            </el-form-item>

            <el-form-item label="背景图片是否重复显示">
                <el-select v-model="config.ui_config.background_repeat" placeholder="no-repeat" style="width: 240px"
                    @change="(val: string) => configStore.updateConfig({ ui_config: { background_repeat: val } })">
                    <el-option v-for="item in background_repeat" :key="item.value" :label="item.label"
                        :value="item.value" />
                </el-select>
            </el-form-item>

            <el-form-item label="背景图片透明度">
                <el-input-number v-model="config.ui_config.background_opacity" placeholder="65" :min="0" :max="1"
                    :step="0.1"
                    @change="(val: number) => configStore.updateConfig({ ui_config: { background_opacity: val } })" />
                <el-tooltip class="box-item" effect="dark" content="取值范围：[0, 1]。1表示完全不透明，0表示完全透明">
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
                <el-tooltip class="box-item" effect="dark" content="如果不会选择结果栏的选中项颜色，可以使用该功能计算背景图片的主题色">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
                <div v-if="dominant_color"> 该图片的主题色为: {{ dominant_color }} </div>
            </el-form-item>
        </el-tab-pane>

        <el-tab-pane label="程序窗口设置" style="height: 100%;overflow-y: auto;">
            <el-form-item label="窗口垂直方向偏移比例因子">
                <el-input-number v-model="config.ui_config.vertical_position_ratio" placeholder="0.4" :min="0"
                    :step="0.05" :max="1"
                    @change="(val: number) => configStore.updateConfig({ ui_config: { vertical_position_ratio: val } })" />
                <el-tooltip class="box-item" effect="dark" content="0表示在屏幕顶部，1表示在屏幕底部，0.5表示在屏幕正中间">
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
                <el-tooltip class="box-item" effect="dark" content="单位px：是数字图像的最小单位，是屏幕显示的基本点">
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

            <el-form-item label="使用windows系统圆角">
                <el-switch v-model="config.ui_config.use_windows_sys_control_radius"
                    @change="(val: boolean) => configStore.updateConfig({ ui_config: { use_windows_sys_control_radius: val } })" />
                <el-tooltip class="box-item" effect="dark" content="仅支持 Windows11 22h2 及以上的版本">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>

            <el-form-item label="窗口的圆角大小">
                <el-input-number v-model="config.ui_config.window_corner_radius" placeholder="8" :min="1" :step="1"
                    :precision="0" :disabled="config.ui_config.use_windows_sys_control_radius"
                    @change="(val: number) => configStore.updateConfig({ ui_config: { window_corner_radius: val } })">
                    <template #suffix>
                        <span>px</span>
                    </template>
                </el-input-number>
                <el-tooltip class="box-item" effect="dark" content="使用系统圆角时，无法更改圆角大小">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>

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
import { onMounted, ref } from 'vue';
import { useRemoteConfigStore } from '../stores/remote_config';
import { storeToRefs } from 'pinia';


const configStore = useRemoteConfigStore()
const { config } = storeToRefs(configStore)

const blur_style_option = [
    {
        value: 'None',
        label: '无效果',
    },
    {
        value: 'Acrylic',
        label: '亚克力效果(Acrylic)',
    },
    {
        value: 'Mica',
        label: '云母效果(Mica)',
    },
    {
        value: 'Tabbed',
        label: '标签式效果(Tabbed)'
    }
]

const background_size = [
    {
        value: 'cover',
        label: '图片占满窗口(cover)',
    },
    {
        value: 'contain',
        label: '图片在窗口内(contain)',
    }, {
        value: 'auto',
        label: '图片保持原尺寸(auto)',
    }
]

const background_position = [
    {
        value: 'center',
        label: '居中(center)',
    },
    {
        value: 'top',
        label: '上对齐(top)',
    }, {
        value: 'bottom',
        label: '下对齐(bottom)',
    }, {
        value: 'left',
        label: '左对齐(left)',
    },
    {
        value: 'right',
        label: '右对齐(right)',
    }, {
        value: 'left top',
        label: '左上角对齐(left top)',
    }, {
        value: 'right top',
        label: '右上角对齐(right top)',
    },
    {
        value: 'left bottom',
        label: '左下角对齐(left bottom)',
    }, {
        value: 'right bottom',
        label: '右下角对齐(right bottom)',
    },
]

const background_repeat = [
    {
        value: 'no-repeat',
        label: '不重复(no-repeat)',
    }, {
        value: 'repeat',
        label: '水平与垂直方向都重复(repeat)',
    }, {
        value: 'repeat-x',
        label: '水平方向重复(repeat-x)',
    }, {
        value: 'repeat-y',
        label: '垂直方向重复(repeat-y)',
    },
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

const systemFonts = ref<string[]>([
    'Default', 'Arial', 'Helvetica', 'Times New Roman', 'Courier New', 'SimSun', 'Microsoft YaHei'
])

// 加载系统字体的方法
const loadSystemFonts = async () => {
    try {
        const fonts = await invoke('command_get_system_fonts')
        if (Array.isArray(fonts) && fonts.length > 0) {
            systemFonts.value = fonts
        }
    } catch (error) {
        console.error('获取系统字体失败:', error)
    }
}

onMounted(async () => {
    await loadSystemFonts()
})

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
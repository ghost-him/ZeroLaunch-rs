<template>
    <el-tabs style="height: 100%;">
        <el-tab-pane :label="t('ui_config.search_and_result_settings')" style="height: 100%;overflow-y: auto;">
            <el-divider content-position="left">{{ t('ui_config.hints') }}</el-divider>
            <el-form-item :label="t('ui_config.custom_search_bar_placeholder')">
                <el-input v-model="config.app_config.search_bar_placeholder" placeholder="Hello, ZeroLaunch!"
                    @change="(val: string) => configStore.updateConfig({ app_config: { search_bar_placeholder: val } })" />
            </el-form-item>

            <el-form-item :label="t('ui_config.custom_footer_tips')">
                <el-input v-model="config.app_config.tips" placeholder="ZeroLaunch-rs v0.4.0"
                    @change="(val: string) => configStore.updateConfig({ app_config: { tips: val } })" />
            </el-form-item>
            <el-divider content-position="left">{{ t('ui_config.background_color') }}</el-divider>
            <el-form-item :label="t('ui_config.overall_background_color')">
                <el-color-picker v-model="config.ui_config.program_background_color" show-alpha
                    @change="(val: string) => configStore.updateConfig({ ui_config: { program_background_color: rgbaToHex(val) } })" />
            </el-form-item>

            <el-form-item :label="t('ui_config.search_bar_status_bar_background')">
                <el-color-picker v-model="config.ui_config.search_bar_background_color" show-alpha
                    @change="(val: string) => configStore.updateConfig({ ui_config: { search_bar_background_color: rgbaToHex(val) } })" />
            </el-form-item>

            <el-form-item :label="t('ui_config.result_bar_highlight_color')">
                <el-color-picker v-model="config.ui_config.selected_item_color" show-alpha
                    @change="(val: string) => configStore.updateConfig({ ui_config: { selected_item_color: rgbaToHex(val) } })" />
            </el-form-item>

            <el-form-item :label="t('ui_config.dark_mode_recommended_colors')">
                <el-tooltip class="box-item" effect="dark" :content="t('ui_config.dark_mode_tooltip')">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>

            <el-divider content-position="left">{{ t('ui_config.search_bar') }}</el-divider>
            <el-form-item :label="t('ui_config.search_bar_font_settings')">
                <el-select v-model="config.ui_config.search_bar_font_family" filterable
                    :placeholder="t('ui_config.select_or_enter_font')"
                    @change="(val: string) => configStore.updateConfig({ ui_config: { search_bar_font_family: val } })">
                    <el-option v-for="font in systemFonts" :key="font" :label="font" :value="font">
                        <span :style="{ fontFamily: font }">{{ font }}</span>
                    </el-option>
                </el-select>
            </el-form-item>
            <el-form-item :label="t('ui_config.search_bar_font_color')">
                <el-color-picker v-model="config.ui_config.search_bar_font_color"
                    @change="(val: string) => configStore.updateConfig({ ui_config: { search_bar_font_color: rgbaToHex(val) } })" />
            </el-form-item>
            <el-form-item :label="t('ui_config.search_bar_placeholder_font_color')">
                <el-color-picker v-model="config.ui_config.search_bar_placeholder_font_color"
                    @change="(val: string) => configStore.updateConfig({ ui_config: { search_bar_placeholder_font_color: rgbaToHex(val) } })" />
            </el-form-item>
            <el-form-item :label="t('ui_config.search_bar_font_size')">
                <el-input-number v-model="config.ui_config.search_bar_font_size" placeholder="50" :min="5" :step="5"
                    :max="100"
                    @change="(val: number) => configStore.updateConfig({ ui_config: { search_bar_font_size: val } })">
                    <template #suffix>
                        <span>%</span>
                    </template>
                </el-input-number>
                <el-tooltip class="box-item" effect="dark" :content="t('ui_config.font_size_tooltip')">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>
            <el-form-item :label="t('ui_config.search_bar_animation')">
                <el-switch v-model="config.ui_config.search_bar_animate"
                    @change="(val: boolean) => configStore.updateConfig({ ui_config: { search_bar_animate: val } })" />
                <el-tooltip class="box-item" effect="dark" :content="t('ui_config.search_bar_animation_tooltip')">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>
            <el-divider content-position="left">{{ t('ui_config.result_bar') }}</el-divider>
            <el-form-item :label="t('ui_config.result_bar_font_settings')">
                <el-select v-model="config.ui_config.result_item_font_family" filterable
                    :placeholder="t('ui_config.select_or_enter_font')"
                    @change="(val: string) => configStore.updateConfig({ ui_config: { result_item_font_family: val } })">
                    <el-option v-for="font in systemFonts" :key="font" :label="font" :value="font">
                        <span :style="{ fontFamily: font }">{{ font }}</span>
                    </el-option>
                </el-select>
            </el-form-item>
            <el-form-item :label="t('ui_config.result_bar_font_color')">
                <el-color-picker v-model="config.ui_config.item_font_color"
                    @change="(val: string) => configStore.updateConfig({ ui_config: { item_font_color: rgbaToHex(val) } })" />
            </el-form-item>
            <el-form-item :label="t('ui_config.result_bar_font_size')">
                <el-input-number v-model="config.ui_config.item_font_size" placeholder="33" :min="5" :step="5"
                    :max="100"
                    @change="(val: number) => configStore.updateConfig({ ui_config: { item_font_size: val } })">
                    <template #suffix>
                        <span>%</span>
                    </template>
                </el-input-number>
                <el-tooltip class="box-item" effect="dark" :content="t('ui_config.result_font_size_tooltip')">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>
            <el-divider content-position="left">{{ t('ui_config.footer_bar') }}</el-divider>
            <el-form-item :label="t('ui_config.footer_font_settings')">
                <el-select v-model="config.ui_config.footer_font_family" filterable
                    :placeholder="t('ui_config.select_or_enter_font')"
                    @change="(val: string) => configStore.updateConfig({ ui_config: { footer_font_family: val } })">
                    <el-option v-for="font in systemFonts" :key="font" :label="font" :value="font">
                        <span :style="{ fontFamily: font }">{{ font }}</span>
                    </el-option>
                </el-select>
            </el-form-item>
            <el-form-item :label="t('ui_config.footer_font_color')">
                <el-color-picker v-model="config.ui_config.footer_font_color"
                    @change="(val: string) => configStore.updateConfig({ ui_config: { footer_font_color: rgbaToHex(val) } })" />
            </el-form-item>
            <el-form-item :label="t('ui_config.footer_font_size')">
                <el-input-number v-model="config.ui_config.footer_font_size" placeholder="33" :min="5" :step="5"
                    :max="100"
                    @change="(val: number) => configStore.updateConfig({ ui_config: { footer_font_size: val } })">
                    <template #suffix>
                        <span>%</span>
                    </template>
                </el-input-number>
                <el-tooltip class="box-item" effect="dark" :content="t('ui_config.footer_font_size_tooltip')">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>
        </el-tab-pane>

        <el-tab-pane :label="t('ui_config.background_image_settings')" style="height: 100%;overflow-y: auto;">
            <el-form-item :label="t('ui_config.blur_effect')">
                <el-select v-model="config.ui_config.blur_style" placeholder="Select" style="width: 240px"
                    :disabled="!config.ui_config.use_windows_sys_control_radius"
                    @change="(val: string) => configStore.updateConfig({ ui_config: { blur_style: val } })">
                    <el-option v-for="item in blur_style_option" :key="item.value" :label="item.label"
                        :value="item.value" />
                </el-select>
                <el-tooltip class="box-item" effect="dark" :content="t('ui_config.system_radius_only')">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>

            <el-form-item :label="t('ui_config.background_image_size')">
                <el-select v-model="config.ui_config.background_size" placeholder="cover" style="width: 240px"
                    @change="(val: string) => configStore.updateConfig({ ui_config: { background_size: val } })">
                    <el-option v-for="item in background_size" :key="item.value" :label="item.label"
                        :value="item.value" />
                </el-select>
                <el-tooltip class="box-item" effect="dark" :content="t('ui_config.background_size_tooltip')">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>

            <el-form-item :label="t('ui_config.background_position')">
                <el-select v-model="config.ui_config.background_position" placeholder="center" style="width: 240px"
                    @change="(val: string) => configStore.updateConfig({ ui_config: { background_position: val } })">
                    <el-option v-for="item in background_position" :key="item.value" :label="item.label"
                        :value="item.value" />
                </el-select>
                <el-tooltip class="box-item" effect="dark" :content="t('ui_config.background_position_tooltip')">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>

            </el-form-item>

            <el-form-item :label="t('ui_config.background_repeat')">
                <el-select v-model="config.ui_config.background_repeat" placeholder="no-repeat" style="width: 240px"
                    @change="(val: string) => configStore.updateConfig({ ui_config: { background_repeat: val } })">
                    <el-option v-for="item in background_repeat" :key="item.value" :label="item.label"
                        :value="item.value" />
                </el-select>
            </el-form-item>

            <el-form-item :label="t('ui_config.background_opacity')">
                <el-input-number v-model="config.ui_config.background_opacity" placeholder="65" :min="0" :max="1"
                    :step="0.1"
                    @change="(val: number) => configStore.updateConfig({ ui_config: { background_opacity: val } })" />
                <el-tooltip class="box-item" effect="dark" :content="t('ui_config.background_opacity_tooltip')">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>

            <el-form-item :label="t('ui_config.select_background_image')">
                <el-button type="primary" @click="select_background_picture">{{ t('ui_config.select_image')
                    }}</el-button>
                <el-button type="danger" @click="delete_background_picture">{{ t('ui_config.delete_image')
                    }}</el-button>

            </el-form-item>
            <el-form-item :label="t('ui_config.calculate_dominant_color')">
                <el-button type="primary" @click="get_dominant_color">{{ t('ui_config.select_image') }}</el-button>
                <el-tooltip class="box-item" effect="dark" :content="t('ui_config.dominant_color_tooltip')">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
                <div v-if="dominant_color"> {{ t('ui_config.dominant_color_result', { color: dominant_color }) }} </div>
            </el-form-item>
        </el-tab-pane>

        <el-tab-pane :label="t('ui_config.window_settings')" style="height: 100%;overflow-y: auto;">
            <el-form-item :label="t('ui_config.vertical_position_ratio')">
                <el-input-number v-model="config.ui_config.vertical_position_ratio" placeholder="0.4" :min="0"
                    :step="0.05" :max="1"
                    @change="(val: number) => configStore.updateConfig({ ui_config: { vertical_position_ratio: val } })" />
                <el-tooltip class="box-item" effect="dark" :content="t('ui_config.vertical_position_tooltip')">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>

            <el-form-item :label="t('ui_config.search_bar_height')">
                <el-input-number v-model="config.ui_config.search_bar_height" placeholder="65" :min="1" :step="1"
                    :precision="0"
                    @change="(val: number) => configStore.updateConfig({ ui_config: { search_bar_height: val } })">
                    <template #suffix>
                        <span>px</span>
                    </template>
                </el-input-number>
                <el-tooltip class="box-item" effect="dark" :content="t('ui_config.px_unit_tooltip')">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>

            <el-form-item :label="t('ui_config.result_item_height')">
                <el-input-number v-model="config.ui_config.result_item_height" placeholder="62" :min="1" :step="1"
                    :precision="0"
                    @change="(val: number) => configStore.updateConfig({ ui_config: { result_item_height: val } })">
                    <template #suffix>
                        <span>px</span>
                    </template>
                </el-input-number>
            </el-form-item>

            <el-form-item :label="t('ui_config.footer_height')">
                <el-input-number v-model="config.ui_config.footer_height" placeholder="42" :min="0" :step="1"
                    :precision="0"
                    @change="(val: number) => configStore.updateConfig({ ui_config: { footer_height: val } })">
                    <template #suffix>
                        <span>px</span>
                    </template>
                </el-input-number>
            </el-form-item>

            <el-form-item :label="t('ui_config.window_width')">
                <el-input-number v-model="config.ui_config.window_width" placeholder="1000" :min="400" :step="1"
                    :precision="0"
                    @change="(val: number) => configStore.updateConfig({ ui_config: { window_width: val } })">
                    <template #suffix>
                        <span>px</span>
                    </template>
                </el-input-number>
            </el-form-item>

            <el-form-item :label="t('ui_config.use_windows_system_radius')">
                <el-switch v-model="config.ui_config.use_windows_sys_control_radius"
                    @change="(val: boolean) => configStore.updateConfig({ ui_config: { use_windows_sys_control_radius: val } })" />
                <el-tooltip class="box-item" effect="dark" :content="t('ui_config.windows11_requirement')">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>
            </el-form-item>

            <el-form-item :label="t('ui_config.window_corner_radius')">
                <el-input-number v-model="config.ui_config.window_corner_radius" placeholder="8" :min="1" :step="1"
                    :precision="0" :disabled="config.ui_config.use_windows_sys_control_radius"
                    @change="(val: number) => configStore.updateConfig({ ui_config: { window_corner_radius: val } })">
                    <template #suffix>
                        <span>px</span>
                    </template>
                </el-input-number>
                <el-tooltip class="box-item" effect="dark" :content="t('ui_config.system_radius_disabled')">
                    <el-icon class="el-question-icon">
                        <QuestionFilled />
                    </el-icon>
                </el-tooltip>

            </el-form-item>
        </el-tab-pane>
    </el-tabs>


</template>

<script lang="ts" setup>
import { useI18n } from 'vue-i18n';
import { rgbaToHex } from '../utils/color';
import { QuestionFilled } from '@element-plus/icons-vue'
import { invoke } from '@tauri-apps/api/core';
import { ElMessage } from 'element-plus';
import { open } from '@tauri-apps/plugin-dialog';
import { onMounted, ref, computed } from 'vue';
import { useRemoteConfigStore } from '../stores/remote_config';
import { storeToRefs } from 'pinia';

const { t } = useI18n();


const configStore = useRemoteConfigStore()
const { config } = storeToRefs(configStore)

const blur_style_option = computed(() => [
    {
        value: 'None',
        label: `${t('ui_config.blur_style_none')}(None)`,
    },
    {
        value: 'Acrylic',
        label: `${t('ui_config.blur_style_acrylic')}(Acrylic)`,
    },
    {
        value: 'Mica',
        label: `${t('ui_config.blur_style_mica')}(Mica)`,
    },
    {
        value: 'Tabbed',
        label: `${t('ui_config.blur_style_tabbed')}(Tabbed)`
    }
])

const background_size = computed(() => [
    {
        value: 'cover',
        label: `${t('ui_config.background_size_cover')}(cover)`,
    },
    {
        value: 'contain',
        label: `${t('ui_config.background_size_contain')}(contain)`,
    }, {
        value: 'auto',
        label: `${t('ui_config.background_size_auto')}(auto)`,
    }
])

const background_position = computed(() => [
    {
        value: 'center',
        label: `${t('ui_config.background_position_center')}(center)`,
    },
    {
        value: 'top',
        label: `${t('ui_config.background_position_top')}(top)`,
    }, {
        value: 'bottom',
        label: `${t('ui_config.background_position_bottom')}(bottom)`,
    }, {
        value: 'left',
        label: `${t('ui_config.background_position_left')}(left)`,
    },
    {
        value: 'right',
        label: `${t('ui_config.background_position_right')}(right)`,
    }, {
        value: 'left top',
        label: `${t('ui_config.background_position_left_top')}(left top)`,
    }, {
        value: 'right top',
        label: `${t('ui_config.background_position_right_top')}(right top)`,
    },
    {
        value: 'left bottom',
        label: `${t('ui_config.background_position_left_bottom')}(left bottom)`,
    }, {
        value: 'right bottom',
        label: `${t('ui_config.background_position_right_bottom')}(right bottom)`,
    },
])

const background_repeat = computed(() => [
    {
        value: 'no-repeat',
        label: `${t('ui_config.background_repeat_no_repeat')}(no-repeat)`,
    }, {
        value: 'repeat',
        label: `${t('ui_config.background_repeat_repeat')}(repeat)`,
    }, {
        value: 'repeat-x',
        label: `${t('ui_config.background_repeat_repeat_x')}(repeat-x)`,
    }, {
        value: 'repeat-y',
        label: `${t('ui_config.background_repeat_repeat_y')}(repeat-y)`,
    },
])

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
        console.error(t('ui_config.get_system_fonts_failed'), error)
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
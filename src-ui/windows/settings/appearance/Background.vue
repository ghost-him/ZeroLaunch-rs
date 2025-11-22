<template>
  <div class="settings-page">
    <h2 class="page-title">
      {{ t('ui_config.background_image_settings') }}
    </h2>
    <div class="content-container">
      <el-form label-width="auto">
        <el-form-item :label="t('ui_config.blur_effect')">
          <el-select
            v-model="config.ui_config.blur_style"
            placeholder="Select"
            style="width: 240px"
            :disabled="!config.ui_config.use_windows_sys_control_radius"
            @change="(val: string) => configStore.updateConfig({ ui_config: { blur_style: val } })"
          >
            <el-option
              v-for="item in blur_style_option"
              :key="item.value"
              :label="item.label"
              :value="item.value"
            />
          </el-select>
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('ui_config.system_radius_only')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>

        <el-form-item :label="t('ui_config.background_image_size')">
          <el-select
            v-model="config.ui_config.background_size"
            placeholder="cover"
            style="width: 240px"
            @change="(val: string) => configStore.updateConfig({ ui_config: { background_size: val } })"
          >
            <el-option
              v-for="item in background_size"
              :key="item.value"
              :label="item.label"
              :value="item.value"
            />
          </el-select>
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('ui_config.background_size_tooltip')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>

        <el-form-item :label="t('ui_config.background_position')">
          <el-select
            v-model="config.ui_config.background_position"
            placeholder="center"
            style="width: 240px"
            @change="(val: string) => configStore.updateConfig({ ui_config: { background_position: val } })"
          >
            <el-option
              v-for="item in background_position"
              :key="item.value"
              :label="item.label"
              :value="item.value"
            />
          </el-select>
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('ui_config.background_position_tooltip')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>

        <el-form-item :label="t('ui_config.background_repeat')">
          <el-select
            v-model="config.ui_config.background_repeat"
            placeholder="no-repeat"
            style="width: 240px"
            @change="(val: string) => configStore.updateConfig({ ui_config: { background_repeat: val } })"
          >
            <el-option
              v-for="item in background_repeat"
              :key="item.value"
              :label="item.label"
              :value="item.value"
            />
          </el-select>
        </el-form-item>

        <el-form-item :label="t('ui_config.background_opacity')">
          <el-input-number
            v-model="config.ui_config.background_opacity"
            placeholder="65"
            :min="0"
            :max="1"
            :step="0.1"
            @change="(val: number) => configStore.updateConfig({ ui_config: { background_opacity: val } })"
          />
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('ui_config.background_opacity_tooltip')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </el-form-item>

        <el-form-item :label="t('ui_config.select_background_image')">
          <el-button
            type="primary"
            @click="select_background_picture"
          >
            {{ t('ui_config.select_image')
            }}
          </el-button>
          <el-button
            type="danger"
            @click="delete_background_picture"
          >
            {{ t('ui_config.delete_image')
            }}
          </el-button>
        </el-form-item>
                
        <el-form-item :label="t('ui_config.calculate_dominant_color')">
          <el-button
            type="primary"
            @click="get_dominant_color"
          >
            {{ t('ui_config.select_image') }}
          </el-button>
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="t('ui_config.dominant_color_tooltip')"
          >
            <el-icon class="el-question-icon">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
          <div v-if="dominant_color">
            {{ t('ui_config.dominant_color_result', { color: dominant_color }) }}
          </div>
        </el-form-item>
      </el-form>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { useI18n } from 'vue-i18n'
import { QuestionFilled } from '@element-plus/icons-vue'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'
import { open } from '@tauri-apps/plugin-dialog'
import { ref, computed } from 'vue'
import { useRemoteConfigStore } from '../../../stores/remote_config'
import { storeToRefs } from 'pinia'

const { t } = useI18n()

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
        label: `${t('ui_config.blur_style_tabbed')}(Tabbed)`,
    },
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
    },
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
    let file_path = await select_picture()
    if (file_path) {

        invoke('select_background_picture', { path: file_path })
        ElMessage({
            message: '图片已保存',
            type: 'success',
        })
    }
}

const delete_background_picture = () => {
    invoke('select_background_picture', { path: '' })
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
        title: '选择一个图片',         // 文件选择框的标题
        filters: [
            {
                name: 'Images',  // 过滤器的名称
                extensions: ['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp'],  // 允许的图片文件扩展名
            },
        ],
    })
    return file_path
}

let dominant_color = ref<string | null>(null)

const get_dominant_color = async () => {
    let file_path = await select_picture()
    let ret = await invoke<string>('get_dominant_color', { path: file_path })
    dominant_color.value = ret
}
</script>

<style scoped>
.settings-page {
    padding: 20px;
    height: 100%;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
}

.page-title {
    margin-top: 0;
    margin-bottom: 20px;
    font-size: 20px;
    font-weight: 500;
    color: #303133;
}

.content-container {
    flex: 1;
    overflow-y: auto;
}

.el-question-icon {
    margin-left: 8px;
}

.el-icon {
    font-size: 18px;
    color: #606266;
}
</style>

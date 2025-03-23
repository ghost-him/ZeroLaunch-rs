<template>
    <div v-if="visible" class="submenu" ref="submenuRef" :style="submenuStyle" @contextmenu.prevent>
        <div v-for="(item, index) in menuItems" :key="index" class="submenu-item"
            :class="{ 'selected': selectedIndex === index }" @click="handleItemClick(index)" :style="itemStyle">
            <div class="submenu-icon" :style="iconStyle">
                <component :is="item.icon"></component>
            </div>
            <div class="submenu-item-name" :style="nameStyle">
                {{ item.name }}
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue';
import type { Component } from 'vue';

interface MenuItem {
    name: string;
    icon: Component;
    action: Function;
}

interface Position {
    top: number;
    left: number;
}

interface WindowSize {
    width: number;
    height: number;
}

const props = defineProps<{
    itemHeight: number;
    windowSize: WindowSize;
    menuItems: MenuItem[];
    isDark?: boolean;
    cornerRadius?: number;
    hoverColor?: string;
    selectedColor?: string;
    itemFontColor?: string;
    itemFontSizePercent?: number;
}>();

// 组件状态
const visible = ref(false);
const selectedIndex = ref(0);
const submenuRef = ref<HTMLElement | null>(null);
const position = ref<Position>({ top: 0, left: 0 });

// 计算样式
const submenuStyle = computed(() => {
    return {
        top: `${position.value.top}px`,
        left: `${position.value.left}px`,
        border: `1px solid ${props.isDark ? '#3d3d3d' : '#bdbdbd'}`,
        borderRadius: `${(props.cornerRadius || 8) / 2}px`,
        backgroundColor: props.isDark ? '#252525' : '#ffffff',
        boxShadow: '0 2px 12px 0 rgba(0, 0, 0, 0.1)',
        zIndex: 1000,
    };
});

const itemStyle = computed(() => {
    return {
        '--hover-color': props.hoverColor || '#f5f5f5',
        '--selected-color': props.selectedColor || '#e6f7ff',
        height: `${props.itemHeight * 0.6}px`,
        display: 'flex',
        alignItems: 'center',
        cursor: 'pointer',
        padding: '0 8px',
        transition: 'background-color 0.2s',
    };
});

const iconStyle = computed(() => {
    return {
        width: `${props.itemHeight * 0.4}px`,
        height: `${props.itemHeight * 0.4}px`,
        marginLeft: `${props.itemHeight * 0.1}px`,
        marginRight: `${props.itemHeight * 0.1}px`,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
    };
});

const nameStyle = computed(() => {
    const fontSize = Math.min(props.itemHeight * (props.itemFontSizePercent || 14) / 100, props.itemHeight * 0.4) * 0.75;
    return {
        fontSize: `${fontSize}px`,
        color: props.itemFontColor || (props.isDark ? '#e0e0e0' : '#333333'),
        marginRight: `${props.itemHeight * 0.1}px`,
        whiteSpace: 'nowrap',
    };
});

// 计算菜单尺寸
const calculateMenuSize = () => {
    if (!submenuRef.value) return { width: 0, height: 0 };

    return {
        width: submenuRef.value.offsetWidth,
        height: submenuRef.value.offsetHeight
    };
};

// 计算调整后的位置，确保菜单完全在窗口内
const calculateAdjustedPosition = (top: number, left: number) => {

    const menuSize = calculateMenuSize();
    const { width, height } = props.windowSize;
    console.log(left);
    console.log(menuSize);
    console.log(props.windowSize);
    // 确保菜单不会超出右边界
    if (left + menuSize.width > width) {
        left = width - menuSize.width - 5; // 5px 边距
    }

    // 确保菜单不会超出下边界
    if (top + menuSize.height > height) {
        top = height - menuSize.height - 5; // 5px 边距
    }

    // 确保菜单不会超出上边界
    if (top < 0) {
        top = 5; // 5px 边距
    }

    // 确保菜单不会超出左边界
    if (left < 0) {
        left = 5; // 5px 边距
    }
    console.log("目标：" + top + ' ' + left);
    return { top, left };
};

// 对外暴露的方法
const selectNext = () => {
    selectedIndex.value = (selectedIndex.value + 1) % props.menuItems.length;
};

const selectPrevious = () => {
    selectedIndex.value = (selectedIndex.value - 1 + props.menuItems.length) % props.menuItems.length;
};

watch(visible, async (newValue) => {
    if (newValue) {
        await nextTick();
        // 菜单显示后重新计算位置
        const adjustedPosition = calculateAdjustedPosition(position.value.top, position.value.left);
        if (submenuRef.value) {
            position.value.top = adjustedPosition.top;
            position.value.left = adjustedPosition.left;
        }
    }
});

// 修改后的showMenu方法，需要传入位置参数
const showMenu = (newPosition: Position) => {
    initMenu();
    position.value = newPosition;
    const adjustedPosition = calculateAdjustedPosition(position.value.top, position.value.left);
    if (submenuRef.value) {
        position.value.top = adjustedPosition.top;
        position.value.left = adjustedPosition.left;
    }
    visible.value = true;
};

const hideMenu = () => {
    visible.value = false;
};

const selectCurrent = () => {
    if (visible.value && props.menuItems[selectedIndex.value]) {
        props.menuItems[selectedIndex.value].action();
        hideMenu();
    }
};

const initMenu = () => {
    selectedIndex.value = 0;
};

// 处理菜单项点击
const handleItemClick = (index: number) => {
    selectedIndex.value = index;
    selectCurrent();
};

const isVisible = () => {
    return visible.value;
}

// 向外暴露方法
defineExpose({
    selectNext,
    selectPrevious,
    showMenu,
    hideMenu,
    selectCurrent,
    initMenu,
    isVisible
});
</script>

<style scoped>
.submenu {
    position: absolute;
    min-width: 120px;
    overflow: hidden;
}

.submenu-item {
    position: relative;
}

.submenu-item:hover {
    background-color: var(--hover-color);
}

.submenu-item.selected {
    background-color: var(--selected-color);
}

.submenu-icon {
    color: inherit;
}

.submenu-item-name {
    flex: 1;
}
</style>
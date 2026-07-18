import { h } from 'vue'
import { NIcon } from 'naive-ui'
import type { Component } from 'vue'
import {
  Settings,
  Search,
  Puzzle,
  Palette,
  Info,
  Box,
  Bug,
} from 'lucide-vue-next'

const iconMap: Record<string, Component> = {
  settings: Settings,
  search: Search,
  extension: Puzzle,
  palette: Palette,
  info: Info,
  plugin: Box,
  bug: Bug,
}

export function renderSidebarIcon(icon: string) {
  const Comp = iconMap[icon] ?? Box
  return () => h(NIcon, null, { default: () => h(Comp) })
}

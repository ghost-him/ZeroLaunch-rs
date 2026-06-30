import { h } from 'vue'
import { NIcon } from 'naive-ui'

function settingsIcon() {
  return h('svg', { xmlns: 'http://www.w3.org/2000/svg', viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', 'stroke-width': '2' }, [
    h('circle', { cx: '12', cy: '12', r: '3' }),
    h('path', { d: 'M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z' })
  ])
}

function searchIcon() {
  return h('svg', { xmlns: 'http://www.w3.org/2000/svg', viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', 'stroke-width': '2' }, [
    h('circle', { cx: '11', cy: '11', r: '8' }),
    h('path', { d: 'm21 21-4.3-4.3' })
  ])
}

function extensionIcon() {
  return h('svg', { xmlns: 'http://www.w3.org/2000/svg', viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', 'stroke-width': '2' }, [
    h('path', { d: 'M20 12V8H6a2 2 0 0 1-2-2c0-2.2 1.8-4 4-4 .7 0 1.4.2 2 .5' })
  ])
}

function paletteIcon() {
  return h('svg', { xmlns: 'http://www.w3.org/2000/svg', viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', 'stroke-width': '2' }, [
    h('path', { d: 'M12 2a10 10 0 0 0 0 20 10 10 0 0 0 0-20z' }),
    h('path', { d: 'M12 2a10 10 0 0 0 0 20' })
  ])
}

function infoIcon() {
  return h('svg', { xmlns: 'http://www.w3.org/2000/svg', viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', 'stroke-width': '2' }, [
    h('circle', { cx: '12', cy: '12', r: '10' }),
    h('line', { x1: '12', y1: '16', x2: '12', y2: '12' }),
    h('line', { x1: '12', y1: '8', x2: '12.01', y2: '8' })
  ])
}

function pluginIcon() {
  return h('svg', { xmlns: 'http://www.w3.org/2000/svg', viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', 'stroke-width': '2' }, [
    h('rect', { x: '3', y: '3', width: '18', height: '18', rx: '2', ry: '2' })
  ])
}

const iconMap: Record<string, () => ReturnType<typeof h>> = {
  settings: settingsIcon,
  search: searchIcon,
  extension: extensionIcon,
  palette: paletteIcon,
  info: infoIcon,
  plugin: pluginIcon,
}

export function renderSidebarIcon(icon: string) {
  const iconFn = iconMap[icon] ?? iconMap['plugin']
  return () => h(NIcon, null, { default: iconFn })
}

import { defineStore } from 'pinia'
import { ref } from 'vue'
import { darkTheme, type GlobalTheme } from 'naive-ui'

const STORAGE_KEY = 'zerolaunch-theme'

function readStoredTheme(): boolean | null {
  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    if (stored === 'dark') return true
    if (stored === 'light') return false
  } catch { /* localStorage blocked */ }
  return null
}

function detectSystemPreference(): boolean {
  try {
    return window.matchMedia('(prefers-color-scheme: dark)').matches
  } catch {
    return false
  }
}

function getInitialTheme(): boolean {
  const stored = readStoredTheme()
  return stored ?? detectSystemPreference()
}

export const useThemeStore = defineStore('theme', () => {
  const isDark = ref(getInitialTheme())
  const naiveTheme = ref<GlobalTheme | null>(isDark.value ? darkTheme : null)

  if (isDark.value) {
    document.documentElement.classList.add('dark')
  }

  function persist( dark: boolean) {
    try {
      localStorage.setItem(STORAGE_KEY, dark ? 'dark' : 'light')
    } catch { /* localStorage blocked */ }
  }

  function apply(dark: boolean) {
    isDark.value = dark
    naiveTheme.value = dark ? darkTheme : null
    document.documentElement.classList.toggle('dark', dark)
    persist(dark)
  }

  function toggleTheme() {
    apply(!isDark.value)
  }

  function setTheme(dark: boolean) {
    apply(dark)
  }

  return { isDark, naiveTheme, toggleTheme, setTheme }
})

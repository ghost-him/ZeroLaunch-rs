import { defineStore } from 'pinia'
import { ref } from 'vue'
import { darkTheme, type GlobalTheme } from 'naive-ui'

export const useThemeStore = defineStore('theme', () => {
  const isDark = ref(false)
  const naiveTheme = ref<GlobalTheme | null>(null)

  function toggleTheme() {
    isDark.value = !isDark.value
    naiveTheme.value = isDark.value ? darkTheme : null
    document.documentElement.classList.toggle('dark', isDark.value)
  }

  function setTheme(dark: boolean) {
    isDark.value = dark
    naiveTheme.value = dark ? darkTheme : null
    document.documentElement.classList.toggle('dark', dark)
  }

  return { isDark, naiveTheme, toggleTheme, setTheme }
})

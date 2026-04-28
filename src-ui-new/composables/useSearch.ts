import { useSearchStore } from '../stores/search-store'

let debounceTimer: ReturnType<typeof setTimeout> | null = null

export function useSearch() {
  const store = useSearchStore()

  function handleInput(value: string) {
    store.query = value

    if (debounceTimer) clearTimeout(debounceTimer)
    debounceTimer = setTimeout(() => {
      store.doQuery(value)
    }, 150)
  }

  function cleanup() {
    if (debounceTimer) {
      clearTimeout(debounceTimer)
      debounceTimer = null
    }
  }

  return { handleInput, cleanup }
}

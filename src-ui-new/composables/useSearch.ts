import { useSearchStore } from '../stores/search-store'

export function useSearch() {
  const store = useSearchStore()

  function handleInput(value: string) {
    store.query = value
    store.doQuery(value)
  }

  return { handleInput }
}

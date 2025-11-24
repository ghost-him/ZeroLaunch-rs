import { ref, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { ProgramDisplayInfo } from '../api/program'

export function useProgramSearch() {
    const searchKeyword = ref('')
    const loading = ref(false)
    const programList = ref<ProgramDisplayInfo[]>([])
    const iconUrls = ref(new Map<string, string>())
    let searchTimeout: number | undefined

    const loadIcon = async (row: ProgramDisplayInfo) => {
        if (iconUrls.value.has(row.icon_request_json)) return
        try {
            const data = await invoke<number[]>('load_program_icon', { programGuid: row.program_guid })

            // Use Blob to optimize performance and avoid base64 conversion overhead
            const bytes = new Uint8Array(data)
            const blob = new Blob([bytes], { type: 'image/png' })
            const url = URL.createObjectURL(blob)

            iconUrls.value.set(row.icon_request_json, url)
        } catch (e) {
            console.error('Failed to load icon', e)
        }
    }

    const handleSearch = () => {
        if (searchTimeout) clearTimeout(searchTimeout)
        searchTimeout = window.setTimeout(async () => {
            loading.value = true
            try {
                const results = await invoke<ProgramDisplayInfo[]>('command_search_programs_lightweight', {
                    keyword: searchKeyword.value
                })
                programList.value = results
                // Load icons for results
                results.forEach(loadIcon)
            } catch (e) {
                console.error('Search failed', e)
            } finally {
                loading.value = false
            }
        }, 300)
    }

    const getIconUrl = (icon_request_json: string) => {
        return iconUrls.value.get(icon_request_json) || ''
    }

    const refreshIcon = async (program: ProgramDisplayInfo) => {
        const oldUrl = iconUrls.value.get(program.icon_request_json)
        if (oldUrl) {
            URL.revokeObjectURL(oldUrl)
            iconUrls.value.delete(program.icon_request_json)
        }
        await loadIcon(program)
    }

    // Clean up resources
    onUnmounted(() => {
        if (searchTimeout) clearTimeout(searchTimeout)
        iconUrls.value.forEach(url => URL.revokeObjectURL(url))
        iconUrls.value.clear()
    })

    return {
        searchKeyword,
        loading,
        programList,
        handleSearch,
        getIconUrl,
        refreshIcon
    }
}

import { computed, ref, Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export type SearchResultTuple = [number, string, string]

type LaunchMethodKind = 'Path' | 'PackageFamilyName' | 'File' | 'Url' | 'Command'

interface LaunchTemplateInfo {
  template: string
  kind: LaunchMethodKind
  placeholderCount: number
  showName: string
}

interface LaunchTemplateInfoResponse {
  template: string
  kind: LaunchMethodKind
  placeholder_count: number
  show_name: string
}

interface InlineParameterSession {
  programGuid: number
  info: LaunchTemplateInfo
  lockedPrefix: string
}

interface UseInlineParameterModeOptions {
  searchText: Ref<string>
  selectedIndex: Ref<number>
  searchResults: Ref<SearchResultTuple[]>
  latestLaunchProgram: Ref<SearchResultTuple[]>
  isAltPressed: Ref<boolean>
  isEverythingMode: Ref<boolean>
  hasParameterPanelSession: Ref<boolean>
  onWarnEmptyArgs: () => void
  onWarnArgCountMismatch: (payload: {
    expected: number
    actual: number
    parsedArgs: string[]
  }) => void
}

export function useInlineParameterMode(options: UseInlineParameterModeOptions) {
  const {
    searchText,
    selectedIndex,
    searchResults,
    latestLaunchProgram,
    isAltPressed,
    isEverythingMode,
    hasParameterPanelSession,
    onWarnEmptyArgs,
    onWarnArgCountMismatch,
  } = options

  const inlineParameterSession = ref<InlineParameterSession | null>(null)

  const isInlineParameterMode = computed(() => inlineParameterSession.value !== null)

  const getCurrentResults = (): SearchResultTuple[] => {
    return isAltPressed.value ? latestLaunchProgram.value : searchResults.value
  }

  const decodeHtmlEntities = (text: string): string => {
    return text
      .replace(/&nbsp;/g, ' ')
      .replace(/&amp;/g, '&')
      .replace(/&lt;/g, '<')
      .replace(/&gt;/g, '>')
      .replace(/&quot;/g, '"')
      .replace(/&#39;/g, "'")
  }

  const normalizeMenuName = (name: string): string => {
    const withoutTags = name.replace(/<[^>]*>/g, '')
    return decodeHtmlEntities(withoutTags).trim()
  }

  const parseInlineArgs = (raw: string): string[] => {
    const args: string[] = []
    let current = ''
    let escaped = false

    for (let i = 0; i < raw.length; i += 1) {
      const ch = raw[i]

      if (escaped) {
        if (ch === ' ' || ch === '\\') {
          current += ch
        } else {
          current += `\\${ch}`
        }
        escaped = false
        continue
      }

      if (ch === '\\') {
        escaped = true
        continue
      }

      if (ch === ' ') {
        if (current.length > 0) {
          args.push(current)
          current = ''
        }
        continue
      }

      current += ch
    }

    if (escaped) {
      current += '\\'
    }

    if (current.length > 0) {
      args.push(current)
    }

    return args
  }

  const clearInlineParameterSession = () => {
    inlineParameterSession.value = null
  }

  const isWithinLockedPrefix = (input: string): boolean => {
    const session = inlineParameterSession.value
    if (!session) {
      return false
    }
    return input.startsWith(session.lockedPrefix)
  }

  const tryEnterInlineParameterMode = async (newVal: string): Promise<boolean> => {
    if (inlineParameterSession.value || hasParameterPanelSession.value || isEverythingMode.value) {
      return false
    }

    if (!newVal.endsWith(' ') || newVal.length === 0) {
      return false
    }

    const commandText = newVal.slice(0, -1)
    if (commandText.length === 0) {
      return false
    }

    const currentResults = getCurrentResults()
    const selected = currentResults[selectedIndex.value]
    if (!selected) {
      return false
    }

    const displayName = normalizeMenuName(selected[1])
    if (displayName !== commandText) {
      return false
    }

    try {
      const info = await invoke<LaunchTemplateInfoResponse>('get_launch_template_info', {
        programGuid: selected[0],
      })

      if (info.placeholder_count <= 0) {
        return false
      }

      inlineParameterSession.value = {
        programGuid: selected[0],
        info: {
          template: info.template,
          kind: info.kind,
          placeholderCount: info.placeholder_count,
          showName: info.show_name,
        },
        lockedPrefix: newVal,
      }
      return true
    } catch (error) {
      console.warn('Failed to enter inline parameter mode:', error)
      return false
    }
  }

  const tryLaunchInlineParameters = async (ctrlKey: boolean, shiftKey: boolean): Promise<boolean> => {
    const session = inlineParameterSession.value
    if (!session) {
      return false
    }

    if (!isWithinLockedPrefix(searchText.value)) {
      clearInlineParameterSession()
      return true
    }

    const rawArgs = searchText.value.slice(session.lockedPrefix.length)
    const parsedArgs = parseInlineArgs(rawArgs)
    if (parsedArgs.length === 0) {
      onWarnEmptyArgs()
      return true
    }

    if (parsedArgs.length !== session.info.placeholderCount) {
      onWarnArgCountMismatch({
        expected: session.info.placeholderCount,
        actual: parsedArgs.length,
        parsedArgs,
      })
      return true
    }

    try {
      await invoke('launch_program', {
        programGuid: session.programGuid,
        ctrl: ctrlKey,
        shift: shiftKey,
        args: parsedArgs,
        queryText: searchText.value,
      })
    } catch (error) {
      console.error('Failed to launch program with inline arguments:', error)
    } finally {
      clearInlineParameterSession()
    }

    return true
  }

  return {
    isInlineParameterMode,
    clearInlineParameterSession,
    isWithinLockedPrefix,
    tryEnterInlineParameterMode,
    tryLaunchInlineParameters,
    getCurrentResults,
  }
}

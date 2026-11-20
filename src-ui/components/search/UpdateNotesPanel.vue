<template>
  <div ref="rootRef" class="update-notes" tabindex="-1">
    <header class="update-notes-header">
      <h2 class="update-notes-title">{{ t('search.updateNotes.title') }}</h2>
      <p class="update-notes-desc">
        {{ t('search.updateNotes.desc', { from: notes?.lastVersion ?? '', to: notes?.currentVersion ?? '' }) }}
      </p>
    </header>

    <div class="update-notes-body">
      <section v-for="item in versions" :key="item.version" class="update-notes-version">
        <div class="update-notes-version-head">
          <span class="update-notes-tag">v{{ item.version }}</span>
          <span v-if="item.releasedAt" class="update-notes-date">{{ item.releasedAt }}</span>
        </div>

        <div v-if="item.summary" class="update-notes-summary">
          <template v-for="(block, index) in parseSummary(item.summary)" :key="index">
            <ul v-if="block.kind === 'list'" class="update-notes-list">
              <li v-for="(segments, itemIndex) in block.items" :key="itemIndex">
                <InlineText :segments="segments" />
              </li>
            </ul>
            <blockquote v-else-if="block.kind === 'quote'" class="update-notes-quote">
              <InlineText :segments="block.segments" />
            </blockquote>
            <p v-else class="update-notes-paragraph">
              <InlineText :segments="block.segments" />
            </p>
          </template>
        </div>

        <div v-for="section in item.sections" :key="section.title" class="update-notes-section">
          <h3 class="update-notes-section-title">{{ section.title }}</h3>
          <ul class="update-notes-list">
            <li v-for="(entry, entryIndex) in section.items" :key="entryIndex">
              <InlineText :segments="parseInline(entry)" />
            </li>
          </ul>
        </div>
      </section>
    </div>

    <footer class="update-notes-footer">
      <span class="update-notes-hint">{{ t('search.updateNotes.hint') }}</span>
      <div class="update-notes-actions">
        <n-button size="small" type="primary" @click="onDismiss">
          {{ t('search.updateNotes.dismiss') }}
        </n-button>
      </div>
    </footer>
  </div>
</template>

<script setup lang="ts">
import { computed, defineComponent, h, nextTick, onMounted, ref, type PropType } from 'vue'
import { useI18n } from 'vue-i18n'
import { NButton } from 'naive-ui'
import { useSearchStore } from '../../stores/search-store'

/** 行内片段：把 `**加粗**` 与 `` `代码` `` 拆成可安全渲染的文本段，不引入 HTML。 */
interface InlineSegment {
  /** 片段原文（已去掉标记符号）。 */
  text: string
  /** 是否为加粗片段。 */
  strong: boolean
  /** 是否为行内代码片段。 */
  code: boolean
}

/** 摘要块：把 AI 摘要的 Markdown 子集解析成段落 / 列表 / 引用三种形态。 */
type SummaryBlock =
  | { kind: 'paragraph'; segments: InlineSegment[] }
  | { kind: 'list'; items: InlineSegment[][] }
  | { kind: 'quote'; segments: InlineSegment[] }

/** 行内片段渲染器：只输出文本、strong、code，不做 HTML 注入。 */
const InlineText = defineComponent({
  props: {
    segments: { type: Array as PropType<InlineSegment[]>, required: true },
  },
  setup(props) {
    return () =>
      props.segments.map((segment, index) => {
        if (segment.strong) return h('strong', { key: index }, segment.text)
        if (segment.code) return h('code', { key: index }, segment.text)
        return h('span', { key: index }, segment.text)
      })
  },
})

const store = useSearchStore()
const { t } = useI18n()

const emit = defineEmits<{ (e: 'dismissed'): void }>()

const rootRef = ref<HTMLElement | null>(null)
const notes = computed(() => store.updateNotes)
const versions = computed(() => notes.value?.versions ?? [])

/// 解析行内标记：`**加粗**` 与 `` `代码` ``；其余按纯文本输出。
function parseInline(text: string): InlineSegment[] {
  const segments: InlineSegment[] = []
  const pattern = /(\*\*[^*]+\*\*|`[^`]+`)/g
  let cursor = 0
  for (const match of text.matchAll(pattern)) {
    const index = match.index ?? 0
    if (index > cursor) segments.push({ text: text.slice(cursor, index), strong: false, code: false })
    const raw = match[0]
    if (raw.startsWith('**')) {
      segments.push({ text: raw.slice(2, -2), strong: true, code: false })
    } else {
      segments.push({ text: raw.slice(1, -1), strong: false, code: true })
    }
    cursor = index + raw.length
  }
  if (cursor < text.length) segments.push({ text: text.slice(cursor), strong: false, code: false })
  return segments
}

/// 解析摘要正文：空行分段，`- ` 为列表，`> ` 为引用；其余行并入段落。
function parseSummary(markdown: string): SummaryBlock[] {
  const blocks: SummaryBlock[] = []
  let paragraph: string[] = []
  let list: InlineSegment[][] = []
  let quote: string[] = []

  const flush = () => {
    if (paragraph.length > 0) {
      blocks.push({ kind: 'paragraph', segments: parseInline(paragraph.join(' ')) })
      paragraph = []
    }
    if (list.length > 0) {
      blocks.push({ kind: 'list', items: list })
      list = []
    }
    if (quote.length > 0) {
      blocks.push({ kind: 'quote', segments: parseInline(quote.join(' ')) })
      quote = []
    }
  }

  for (const rawLine of markdown.split(/\r?\n/)) {
    const line = rawLine.trim()
    if (!line) {
      flush()
      continue
    }
    if (line.startsWith('- ')) {
      if (paragraph.length > 0 || quote.length > 0) flush()
      list.push(parseInline(line.slice(2)))
    } else if (/^\d+\.\s/.test(line)) {
      // AI 摘要的有序结构行各自成段，避免相邻编号挤在同一行。
      flush()
      blocks.push({ kind: 'paragraph', segments: parseInline(line) })
    } else if (line.startsWith('> ')) {
      if (paragraph.length > 0 || list.length > 0) flush()
      quote.push(line.slice(2))
    } else {
      if (list.length > 0 || quote.length > 0) flush()
      paragraph.push(line)
    }
  }
  flush()
  return blocks
}

/// 关闭页面并落已读版本：之后不再自动显示本次更新内容。
async function onDismiss() {
  await store.dismissUpdateNotes()
  emit('dismissed')
}

onMounted(async () => {
  await nextTick()
  rootRef.value?.focus()
})
</script>

<style scoped>
.update-notes {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 22px 28px 16px;
  outline: none;
}

.update-notes-header {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.update-notes-title {
  margin: 0;
  font-size: var(--font-size-lg);
  font-weight: 600;
  color: var(--text-primary);
}

.update-notes-desc {
  margin: 0;
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}

.update-notes-body {
  display: flex;
  flex-direction: column;
  gap: 18px;
  max-height: 340px;
  overflow-y: auto;
  padding-right: 6px;
}

.update-notes-version {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.update-notes-version-head {
  display: flex;
  align-items: baseline;
  gap: 8px;
}

.update-notes-tag {
  font-size: var(--font-size-md);
  font-weight: 600;
  color: var(--text-primary);
}

.update-notes-date {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}

.update-notes-summary {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.update-notes-paragraph,
.update-notes-quote {
  margin: 0;
  font-size: var(--font-size-sm);
  line-height: 1.6;
  color: var(--text-primary);
}

.update-notes-quote {
  padding-left: 10px;
  border-left: 2px solid var(--border-color);
  color: var(--text-secondary);
}

.update-notes-section {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.update-notes-section-title {
  margin: 0;
  font-size: var(--font-size-sm);
  font-weight: 600;
  color: var(--text-primary);
}

.update-notes-list {
  margin: 0;
  padding-left: 18px;
  display: flex;
  flex-direction: column;
  gap: 2px;
  font-size: var(--font-size-sm);
  line-height: 1.6;
  color: var(--text-secondary);
}

.update-notes-list code {
  font-size: var(--font-size-xs);
}

.update-notes-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding-top: 12px;
  border-top: 1px solid var(--border-color);
}

.update-notes-hint {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}

.update-notes-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;
}
</style>

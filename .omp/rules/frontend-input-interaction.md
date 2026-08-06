---
description: 前端输入交互层判定规范 — queryStillInPanel/hasTranslateContent 等镜像判定的边界与同步契约
condition: "queryStillInPanel|hasTranslateContent|panelTriggerKeywords|镜像后端"
scope: "tool:edit(src-ui/stores/search-store.ts), tool:write(src-ui/stores/search-store.ts), tool:edit(src-ui/plugins/built-in/translator-panel/TranslationPanel.vue), tool:write(src-ui/plugins/built-in/translator-panel/TranslationPanel.vue), tool:edit(src-ui/plugins/built-in/calculator-panel/CalculatorPanel.vue), tool:write(src-ui/plugins/built-in/calculator-panel/CalculatorPanel.vue)"
---

# 前端输入交互层判定

按 RULES.md「前后端职责边界」（业务逻辑的判据是**权威性**），以下判定属于**输入交互层判定**（用户交互的一部分，非业务逻辑前移），允许存在于前端：

- `queryStillInPanel`（search-store.ts）：镜像后端 `SessionDispatcher::match_trigger`，供防抖豁免与面板查询在途提示
- `hasTranslateContent`（TranslationPanel.vue）：镜像后端 `parse_search_term` 的 @ 语言码剥离，供派发时刻「已开始翻译」提示

约束（违反即违规）：

1. 仅用于 IPC 前时序/UX 决策；权威路由与解析 MUST 仍由后端响应裁决，前端判定不得成为任何状态的最终依据
2. 判定参数 MUST 来自后端下发（如 `session-state` 事件的 `triggerKeywords`），禁止前端硬编码业务数据（语言目录、触发词表、评分公式等）
3. 镜像点 MUST 在注释中声明（如「镜像后端 match_trigger」），后端解析变更时 MUST 同步更新镜像
4. 禁止借「输入交互层判定」之名新增权威逻辑（评分、路由、持久化、平台操作）

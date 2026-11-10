# Issue draft — TTSR for DeepSeek Harness

> Repo: https://github.com/can1357/oh-my-pi · License: MIT
> Paste the English body below into a new issue. Chinese translation follows for review.

---

## Title (EN)

**Would you want to write a TTSR plugin for DeepSeek Harness — or mind if I do?**

## Body (EN)

Hi Can,

TTSR is the one part of omp I miss since I started using DeepSeek Harness. DSH has no rules layer at all: only directory-scoped `AGENTS.md` files plus a plugin API, nothing that watches generation and corrects it mid-stream.

Two questions:

1. **Is a TTSR plugin for DSH something you'd want to build?** I read `docs/ttsr-injection-lifecycle.md` and my honest read is that a faithful port needs the host to abort a generation and re-run it with an extra message. DSH's `agent/pre-step` hook can only approximate that at step granularity, so it would be step-level TTSR, not token-level. If you think it's more tractable than that, I'd rather hear it from you than guess.

2. **If you don't have the time, I can write it myself.** I'd credit it as a port of omp's TTSR, link back to this repo, and keep the rule frontmatter compatible so existing `.omp/rules/*.md` packs load unchanged.

No maintenance expectation either way — I just wanted to ask before building on the idea.

Thanks!

---

## 中文翻译

### 标题

**你愿意为 DeepSeek Harness 写一个 TTSR 插件吗——或者介意我来写吗？**

### 正文

Can 你好，

自从开始用 DeepSeek Harness，omp 里我唯一想念的就是 TTSR。DSH 完全没有规则层：只有目录级的 `AGENTS.md` 加一套插件 API，没有任何东西去盯生成过程并中途纠偏。

两个问题：

1. **为 DSH 写一个 TTSR 插件，是你想做的事吗？** 我读了 `docs/ttsr-injection-lifecycle.md`，我的判断是忠实移植需要宿主持有「中止本次生成、带一条追加消息重跑」的能力。DSH 的 `agent/pre-step` 只能在步骤粒度上近似做到，所以它会是步骤级 TTSR，而不是 token 级。如果你认为它比这更可行，我更愿意听你说，而不是自己猜。

2. **如果你没有时间，我可以自己写。** 我会标明它是 omp TTSR 的移植、链接回本仓库，并保持规则 frontmatter 兼容，让现有 `.omp/rules/*.md` 规则包原样加载。

两种情况下都不期待你维护什么——我只是想在这个想法上动手之前先问一声。

谢谢！

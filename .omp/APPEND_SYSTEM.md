## ZeroLaunch-rs Harness Enhancements

These instructions supplement the OMP base harness. The default OMP tool policy, delegation rules, execution workflow, and delivery contract remain in full effect. Write only what OMP does not already cover.

---

### 1. Code Change Precision

**OMP edit semantics** (reminder — already in the tool descriptions):
- `edit` has multiple modes (`hashline`, `replace`, `patch`, `apply_patch`). Follow the schema exposed in the current session instead of assuming one payload shape.
- Do not use tool/field names from training data or another edit variant (`old_str`, `old_string`, `replace_all`, etc.); follow the current OMP tool schema.

**Respect the active read/edit format:**
- OMP's `read` tool may emit HL mode output like `[filename#TAG]` + `41:def alpha():`, or line-number mode output like `41|def alpha():`. These prefixes are metadata, not file content.
- In hashline mode, copy `[filename#TAG]` for anchored edits, use bare line numbers, and NEVER fabricate the tag.
- In text-matching variants such as `replace`, strip prefixes like `41|` / `41:` before pasting file content into the edit payload.

**Do not re-read to verify a successful edit:**
- `edit` / `write` error on failure; a successful return IS the confirmation. Do not re-read a file just to check that your edit applied. (OMP: "NEVER re-audit an applied edit".)

**Compilation feedback loop (Rust) — project-specific:**
- After a non-trivial change to `.rs` files, run `cargo check` via the `bash` tool before yielding. This is build verification (part of the Verify phase), NOT an audit of the edit.
- If `cargo check` fails on files you modified, the failure is blocking — fix it.
- You can also use `lsp diagnostics` to get in-editor error locations.
- `src-ui/` TypeScript changes are exempt (separate `bun`/`vite` build, no Rust compile impact).

---

### 2. Code Quality Patterns

**Match the surrounding code:**
- Write code that reads like the code around it: match its comment density, naming, and idiom.
- In this project: Rust comments are in Chinese (精简中文注释). Match this convention.

**Act on sufficient information:**
- When you have enough information to act, act. Do not re-derive facts already established in the conversation, re-litigate a decision the user has already made, or narrate options you will not pursue.
- If weighing a choice, give a recommendation, not an exhaustive survey of alternatives.

**Report outcomes faithfully:**
- If `cargo check` fails, say so with the output. If a test was skipped, say that.
- When something is done and verified, state it plainly without hedging.
- Rust compilation and test results are ground truth — report them verbatim.

**Irreversible and outward-facing actions:**
- For actions that are hard to reverse or outward-facing, confirm first unless durably authorized or explicitly told to proceed without asking. Approval in one context does not extend to the next.
- Sending content to an external service publishes it; it may be cached or indexed even if later deleted — that is why confirmation matters.
- Before deleting or overwriting, look at the target. If what you find contradicts how it was described, or you didn't create it, surface that instead of proceeding.
- Within the workspace, the existing OMP guard ("Ask before destructive commands or deleting code you didn't write") applies.

**Denied or failed tool calls:**
- A denied or failed tool call means the approach was rejected — adjust and retry differently, do not re-issue the same call verbatim.

---

### 3. Communication Standards

**Formatting:**
- Use backticks for ALL file paths, directory names, function names, class names, and code symbols. Example: `src-tauri/src/commands/bridge.rs`, `ConfigManager`, `apply_settings()`.
- Reference code as `file_path:line_number` — it is clickable. Example: `src-tauri/src/commands/bridge.rs:12` (line 12), `src-tauri/src/commands/bridge.rs:12-15` (lines 12–15).

**Conciseness:**
- Be concise and direct. Minimize output tokens.
- Answer the specific query; avoid tangential information unless critical to the task.
- When facts are established in context, build on them rather than re-stating them.

---

### 4. Project Awareness

**This is a Tauri 2.x + Vue 3 desktop application (ZeroLaunch) for Windows.**

**Cargo workspace — cross-crate impact is the rule, not the exception:**
- 6 crates + src-tauri + src-ui. A `pub` change in any crate can ripple through dependents.
- `crates/plugin-api` changes affect `plugin-host`, `plugin-sdk-rust`, AND `src-tauri`.

**Compilation is the minimum bar:**
- `cargo check` must pass with zero errors after any Rust change.
- Platform-specific code in `crates/platform-windows/` is normal — this is Windows-only.

**Rules system — TTSR auto-injects domain rules:**
- `.omp/RULES.md` is always in context (engineering discipline).
- `.omp/rules/*.md` are TTSR rules. They auto-inject when you `read`, `edit`, or `write` files matching their scope globs. You do NOT need to manually `read rule://<name>`.
- If a rule failed to auto-inject and you see a gap, only then `read rule://<name>` manually.

**Dependency:** This assumes `omp-rules-migration-plan.md` Phase 1 and Phase 2 are complete. If TTSR rules still use `globs` frontmatter instead of `condition`+`scope`, they will NOT auto-inject and must be read manually.

# Code Review Context — Working Tree vs HEAD

## Summary
This is a **configuration/doc migration** — no application code (Rust/TypeScript/Vue) changed.
- **Files changed**: 25 files, +28 -3009
- **Subsystems**: only `other` (documentation/config)
- **Core subsystems**: none touched

## What Changed

1. **Deleted**: Entire `.claude/` directory (9 rule files + 2 settings files + 4 skill directories) — 23 files, 0 insertions, ~3009 deletions
2. **Added (untracked)**: Entire `.omp/` directory — `AGENTS.md`, `RULES.md`, `config.yml`, `mcp.json`, 7 rule files in `rules/`, 4 skill directories in `skills/`
3. **Modified**: `CLAUDE.md` — simplified from 89 lines to 11 lines, points to `.omp/AGENTS.md`
4. **Modified**: `CONTRIBUTING.md` — updated references from `.claude/` to `.omp/`

## Key Findings (Pre-Analysis)

### Content Migration Completeness
- **Old `.claude/rules/` (8 files)** → migrated to `.omp/rules/` (7 files) + `.omp/RULES.md` (1 file). The old `directory-map.md` content is merged into `.omp/AGENTS.md`.
- **Old `.claude/settings.json`** → migrated to `.omp/config.yml`.
- **Old `.claude/skills/` (4 skills)** → migrated to `.omp/skills/` (same 4 skills).
- **Old `.claude/settings.local.json`** → intentionally not carried over (user-specific, shouldn't be in VCS).

### Content Discrepancies Found

1. **`config.yml` permissions drift**: Old `.claude/settings.json` had 8 CodeGraph MCP permissions; new `.omp/config.yml` only has 1 (`mcp__codegraph__codegraph_explore`). Missing: `codegraph_search`, `codegraph_node`, `codegraph_callers`, `codegraph_callees`, `codegraph_impact`, `codegraph_files`, `codegraph_status`. New also adds several `Bash(...)` permissions not in old.

2. **`RULES.md` wording change**: Old used `**必须**` (Chinese bold); new uses RFC 2119 `MUST`. This is a style improvement but functionally equivalent.

3. **Rule file sizes nearly identical** (diff within ±3 lines), suggesting content is essentially preserved with minor formatting changes.

### What NOT Affected
- No `.rs`, `.ts`, `.vue`, `.js`, `.json` (config), `.toml` files changed
- No IPC commands added/removed/modified
- No dependencies changed
- No build system changes
- No hook, workflow, or CI changes

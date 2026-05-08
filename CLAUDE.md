# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

ZeroLaunch-rs is a Windows application launcher built with Tauri 2.x (Rust) + Vue 3 (TypeScript). It features pinyin/fuzzy matching, local AI semantic search (optional), and Everything SDK integration. The project is currently mid-refactoring on the `refactor/plugin-system` branch — migrating from a monolithic command-based architecture to a layered plugin system.

## Build & Development Commands

### Prerequisites
- Rust v1.90.0+, Bun v1.2.22+

### Frontend
```bash
bun install              # Install frontend dependencies
bun run dev              # Vite dev server (frontend only)
bun run build            # Type-check + build frontend
```

### Tauri (Full App)
```bash
bun run tauri dev        # Dev mode with hot-reload
```

### Production Builds (via xtask)
```bash
cd xtask
cargo run --bin xtask build-installer --arch x64           # AI version (default)
cargo run --bin xtask build-installer --arch x64 --ai disabled  # Lite version
cargo run --bin xtask build-all                            # All variants
cargo run --bin xtask clean                                # Clean build artifacts
```

### Rust Code Quality
```bash
just style               # Run clippy --fix + cargo fmt (in src-tauri/)
# Or manually:
cd src-tauri && cargo clippy --fix --allow-dirty && cargo fmt --all
cargo check              # Verify compilation
```

### Git Hooks
Husky + commitlint enforce conventional commits (`feat:`, `fix:`, `docs:`, `refactor:`, etc.).

## Cargo Features

| Feature | Purpose |
|---------|---------|
| `default` (custom-protocol) | Standard build with Tauri custom protocol |
| `portable` | Portable edition (data stored next to binary) |
| `ai` | Bundles `ort` (ONNX Runtime), `tokenizers`, `ndarray` for local AI embedding |

The Everything SDK (`everything-rs`) is only available on `cfg(target_arch = "x86_64")`.

## Architecture

### Three-Layer Design (New System)

```
┌──────────────────────────────────┐
│  Plugin / PluginSystem           │  ← Business logic: plugins, pipelines, dispatch
│  (plugin/, plugin_system/)       │
├──────────────────────────────────┤
│  Core (core/)                    │  ← ConfigManager, Configurable trait, types
├──────────────────────────────────┤
│  SDK (sdk/)                      │  ← HostApi, platform abstractions, traits
└──────────────────────────────────┘
```

**Dependency rule**: `sdk/ → core/ → plugin/` — no reverse dependencies. SDK never imports core or plugin. Core never imports plugin/plugin_system.

### Key Modules

- **`sdk/`** — `HostApi` is the central hub holding all platform implementations (icon, shell, window, hotkey, app enumeration, etc.). Plugins access platform services through `PluginHandle` (obtained via `HostApi::register()`). Platform-specific code lives in `sdk/platform/windows/`.
- **`core/config/`** — `ConfigManager` orchestrates all `Configurable` components: registration, settings CRUD, validation, persistence, and event broadcasting. `Configurable` trait is the unified contract for all components (plugins, data sources, search engines, etc.).
- **`plugin/`** — Concrete implementations: data sources (`app_source`, `program_source`, `url_source`, `bookmark_source`, `command_source`), keyword optimizers (pinyin, symbol removal, etc.), search engines (`launchy`, `skim`, `standard`), score boosters, executors, and triggerable plugins (`calculator`, `everything`).
- **`plugin_system/`** — Plugin framework: `SessionRouter` (central request orchestrator), `CandidatePipeline` (collect + optimize candidates), `SearchPipeline` (score + rank + boost), `PluginRegistry`, `QueryDispatcher`, `ExecutorRegistry`.
- **`modules/`** — Legacy system modules (config, everything, ui_controller, version_checker) — being replaced by the new architecture.
- **`commands/`** — Tauri IPC bridge. Legacy commands (per-function) being consolidated into `bridge.rs` (14 generic commands) for the new system.
- **`src-ui/`** — Old Vue 3 + Element Plus frontend. **To be deleted** and replaced with a new Vue 3 + Naive UI frontend (see `NEW_FRONTEND_REQUIREMENTS.md`). Only kept as reference during the migration.

### Data Flow: Search

```
User input → bridge_query → SessionRouter.route_query()
  ├─ Plugin trigger hit → plugin mode
  └─ No trigger → search mode
      └─ SearchPipeline.search(candidates, query)
            ├─ SearchEngine.calculate_scores()
            └─ ScoreBooster.boost() → ranked results
```

### Data Flow: Configuration

```
User changes setting → ConfigManager.apply_settings()
  ├─ validate → apply → on_settings_changed() → broadcast ConfigEvent
  └─ save_to_storage() (local JSON + optional WebDAV sync)
```

## Key Conventions

- **Configurable lifecycle contract** (iron rule): `validate_settings` = pure check only. `apply_settings` = update internal state only (write RwLock). `on_settings_changed` = side effects only (rebuild services, register callbacks, etc.). Never put side effects in `apply_settings`; never put validation only in `apply_settings` (override `validate_settings` instead). See `HotkeyConfigComponent` / `InstallationMonitorConfigComponent` for reference.
- **ConfigAction for pre-save tests**: If a side effect must gate whether config is saved (e.g. WebDAV connectivity test), use `config_actions` + `execute_config_action` — a separate user-triggered action, not embedded in the save flow.
- **JSON numeric values**: Always use `as_f64()` not `as_i64()` — the frontend may send floats, and `as_i64()` will silently return 0.
- **Configurable trait**: `apply_settings(&self, ...)` uses `&self` (immutable); components use internal `RwLock` for interior mutability.
- **Inner pattern**: For `RwLock<Inner>` components, the outer struct only delegates (one-line call through the lock). All business logic lives in `Inner`.
- **ActionExecutor is async**: `ActionExecutor::execute` is `async fn` via `#[async_trait]`. Never use `tauri::async_runtime::block_on` in any executor or async context — it blocks the tokio worker thread. Use `.await` and propagate errors instead of `tokio::spawn` fire-and-forget.
- **RwLock guards must not cross `.await`**: `parking_lot::RwLock*Guard` is `!Send`. Holding one across an `.await` makes the future `!Send` and breaks `#[tauri::command]`. Clone data out of the lock in a block, drop the guard, then `.await`.
- **Dead code**: Delete it immediately. No backup files (Git is the backup). No temporary/legacy markers lingering. Remove from `mod.rs` when deleting.
- **Config component granularity**: Split by functional domain (appearance, storage, hotkey, ranking, etc.). Never make a monolithic `AppConfigComponent` or `UIConfigComponent`.
- **Platform services with callbacks** (hotkey, installation monitor): Use a push-based callback registration pattern — register callbacks via trait methods; events trigger all registered callbacks.
- **Existing architecture documentation**: `AGENTS.md` is the detailed reference for the new system architecture. `DESIGN_DECISIONS.md` records design rationale. `REFACTORING_DESIGN.md` explains the refactoring strategy.

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build                          # build
cargo run                            # run the TUI
cargo test                           # run all tests
cargo fmt --all                      # format (uses rustfmt.toml: Module-level imports, StdExternalCrate grouping)
cargo fmt --all -- --check           # check formatting without applying
cargo clippy -- -D warnings          # lint (CI treats warnings as errors)
```

## Architecture

This is a terminal UI app built with `tui-rs` + `crossterm` on Tokio async runtime. It manages two log lists (Logs, Music Logs) against a REST API, with a Settings page for configuring the base URL and HTTP headers at runtime.

### Threading model

Two concurrent tasks share `Arc<Mutex<App>>`:

1. **UI task** (`lib.rs::start_ui`) — renders frames and forwards input events to `App`. The lock is released between draw and the blocking `events.next().await` call so the IO task can write back results without starvation.
2. **IO task** (`io/handler.rs::IoHandler`) — receives `IoEvent` messages over a `mpsc` channel, performs network I/O via `ApiServiceHandler`, then re-acquires the lock to call `App::finish_list_fetch` / `App::finish_post`.

`App::dispatch` is the only way the UI side sends work to the IO side.

### State machine

`AppState` has two variants: `Init` and `Initialized`. After `IoEvent::Initialize` is handled, `App::initialized()` is called to transition to `Initialized`, which carries:

- `nav: Vec<Page>` — a page stack; the last element is the active view.
- `focus: Vec<Focus>` — a focus stack (Menu / LogsList / LogsPreview).
- `metadata: SharedMetadata` — a `Arc<Mutex<HashMap<String, Arc<dyn Any>>>>` cache shared across pages; log lists are stored here keyed by `metadata_key` (e.g. `"logs"`, `"music_logs"`).
- `settings: RequestSettings` — base URL + headers, editable from the Settings page.

### Input → Command → IoEvent pipeline

`keymap.rs` converts raw `Key` inputs into `Command` values in two passes:
- `resolve_universal` — context-independent (quit, back, route hotkeys `1`–`3`)
- `resolve_contextual(UiContext, Key)` — context-specific bindings

`UiContext` is derived from the current focus and the active page's `sub_ui_context()` (which returns `LogEditor`, `LogSaving`, or `LogResult` when a modal is active).

Commands are dispatched through `App::run_universal` / `App::run_contextual`, which may call `App::dispatch(IoEvent)` to queue network work.

### Pages

Each page implements `Draw` and handles its own command set:

- **`LogListPage`** (`pages/log_list.rs`) — reused for both Logs and Music Logs via `LogListConfig` statics (`LOGS_CONFIG`, `MUSIC_LOGS_CONFIG`). Manages a `LogModal` state machine (None → Editor → Saving → Result). On failed create, dismissing the result modal restores the draft editor.
- **`SettingsPage`** (`pages/settings.rs`) — edits `RequestSettings` (base URL row + header rows). Uses its own modal for cell-level text editing.

### API layer

`ApiServiceHandler` (`api/handler.rs`) wraps `reqwest::Client` with a 10-second timeout. All requests read `base_url` and `headers` from `AppState::settings` at call time (not at startup), so settings changes take effect immediately on the next fetch/post.

### Routes and API paths

All route specs and API path constants live in `app/routing.rs`. When adding a new section, add a `RouteSpec` to `ROUTES`, a `RootRoute` variant, and corresponding path constants / page type.

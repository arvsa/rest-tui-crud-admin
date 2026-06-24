# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build                          # build
cargo run                            # run the TUI
cargo test                           # run all tests
cargo test <test_name>               # run a single test
cargo fmt --all                      # format (uses rustfmt.toml: Module-level imports, StdExternalCrate grouping)
cargo fmt --all -- --check           # check formatting without applying
cargo clippy -- -D warnings          # lint (CI treats warnings as errors)
```

## Configuration

The app is config-driven via two YAML files loaded at startup:

- **`config.yaml`** (from `config.example.yaml`) — `base_url` + `headers` map. Both files support `${VAR_NAME}` placeholders expanded from the environment (loaded via `.env` by `dotenvy`).
- **`models.yaml`** (from `models.example.yaml`) — list of `ModelConfig` entries, each defining a REST resource: `name`, `endpoint` (GET list), optional `create_endpoint`, `update_endpoint`, `delete_endpoint`, `id_field`, `display_field`, and optional `fields` list.

To add a new resource, add an entry to `models.yaml` — no code changes needed.

## Architecture

A terminal UI REST CRUD admin dashboard built with `ratatui` + `crossterm` on Tokio.

### Threading model

Two concurrent tasks share `Arc<tokio::sync::Mutex<App>>` (async mutex):

1. **UI task** (`lib.rs::start_ui`) — renders frames and forwards input events to `App`. The lock is released between draw and `events.next().await` so the IO task can write back results without starvation.
2. **IO task** (`io/handler.rs::IoHandler`) — receives `IoEvent` messages over an `mpsc` channel, performs network I/O via `ApiServiceHandler`, then re-acquires the lock to call `App::finish_fetch` / `App::finish_post` / `App::finish_delete`.

`App::dispatch` / `App::dispatch_sync` are the only way the UI side sends work to the IO side.

### State machine

`AppState` has two variants:

- `Init { config, models }` — initial state before the first `IoEvent::Initialize` is handled.
- `Initialized` — carries:
  - `models: Vec<ModelConfig>` — resource definitions from `models.yaml`
  - `sidebar_cursor: usize` — selected model index
  - `records: Vec<serde_json::Value>` — currently loaded records for the selected model
  - `fetch_state: FetchState` — `Idle | Loading | Error(String)`
  - `table_cursor: usize` — selected record index
  - `popups: Vec<Popup>` — popup stack; last element is the active popup
  - `active: ActiveComponent` — `Sidebar | Main | Popup`
  - `request_config: RequestConfig` — base URL + headers (read-only after init; edit the YAML and restart to change)

### Input → Command pipeline

`keymap.rs` converts `Key` inputs into `Command` values in two passes:
- `resolve_universal(Key)` — context-independent: quit (`q` / `Ctrl+c`), toggle help (`?`)
- `resolve_contextual(ActiveContext, Key)` — context-specific bindings

`ActiveContext` is derived from `active` + `popups.last()`:
- `Sidebar` — `j/k/↑↓` navigate, `l/Enter` select and fetch
- `Main` — `j/k` navigate, `h` back to sidebar, `r` refresh, `n` create, `e/Enter` edit, `d` delete
- `Popup(Form)` — `Tab/BackTab` cycle fields, `Ctrl+s` submit, `Backspace` delete char, `Esc` close
- `Popup(ConfirmDelete)` — `y/Y` confirm, `n/N/Esc` cancel
- `Popup(Help)` — `?/Esc/q` close

### Popups

`popup.rs` defines three variants pushed onto `AppState::popups`:
- `Popup::Form { fields, mode: FormMode::Create | Edit, endpoint, .. }` — shared for create and edit workflows
- `Popup::ConfirmDelete { record_display, record_id, endpoint }` — shown before DELETE
- `Popup::Help` — keybinding reference

### API layer

`ApiServiceHandler` (`api/handler.rs`) wraps `reqwest::Client` with a 10-second timeout per request. All four methods (`get_json`, `post_json`, `put_json`, `delete`) take `headers: &[(String, String)]` read from `AppState::request_config` at call time.

`extract_records` in `io/handler.rs` normalises API responses: if the response is a JSON array it's returned directly; if it's an object, the first array-valued field is used (handles paginated wrappers).

Endpoint templates with `{id}` (e.g. `/posts/{id}`) are resolved by `resolve_endpoint_with_id`; templates without `{id}` get the id appended as `/<id>`.

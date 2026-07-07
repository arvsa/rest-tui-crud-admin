# TUI-admin

A terminal UI dashboard for generic CRUD operations against any REST API — think `lazygit` but for REST resources. Fully config-driven: no code changes needed to work with a new API.

The base of this project was cloned from [plop-tui](https://github.com/ilaborie/plop-tui) by [ilaborie](https://github.com/ilaborie).

Built with [ratatui](https://github.com/ratatui-org/ratatui) + Tokio in Rust.

---

## Features

- Sidebar lists all models defined in `models.yaml`
- Main panel fetches and displays records in a table
- Create, edit, and delete records (each action is optional per model)
- Vim-style modal editing in create/edit form fields (Normal/Insert modes, `hjkl` motions, word/line jumps, `dd`)
- Pagination support (page, offset, or cursor style) with cached page-flip navigation (`L` / `H`)
- Help overlay (`?`) with keybinding reference
- Config-driven: swap `config.yaml` + `models.yaml` to target any REST API

---

## Installation

Requires [Rust](https://rustup.rs/) (stable).

```bash
git clone https://github.com/your-username/rest-tui-crud-admin
cd rest-tui-crud-admin
cargo build --release
# binary is at ./target/release/est-tui-crud-admin
```

---

## Setup

**1. Create your `.env`**

```bash
cp .env.example .env
```

Edit `.env` and fill in your API credentials:

```env
BASE_URL=https://api.example.com
AUTH_TOKEN=your-token-here
BYPASS_TOKEN=your-bypass-token-here   # optional
```

**2. Create `config.yaml`**

```bash
cp config.example.yaml config.yaml
```

`config.yaml` sets the base URL and HTTP headers sent with every request.
Reference `.env` values with `${VAR_NAME}`:

```yaml
base_url: "${BASE_URL}"
headers:
  Content-Type: "application/json"
  Authorization: "Bearer ${AUTH_TOKEN}"
```

**3. Create `models.yaml`**

```bash
cp models.example.yaml models.yaml
```

Define each REST resource you want to manage. Only `endpoint` is required;
action endpoints are optional — omit any your API doesn't support:

```yaml
models:
  - name: "Posts"
    endpoint: "/posts"              # GET  — list records (required)
    create_endpoint: "/posts"       # POST — create
    update_endpoint: "/posts/{id}"  # PUT  — update  ({id} is replaced at runtime)
    delete_endpoint: "/posts/{id}"  # DELETE — delete
    id_field: "id"
    display_field: "title"
    fields: ["title", "body"]       # form fields (auto-detected if omitted)
```

**4. Run**

```bash
cargo run
# or
./target/release/rest-tui-crud-admin
```

---

## Keybindings

### Global

| Key | Action |
|-----|--------|
| `?` | Toggle help overlay |
| `q` / `Ctrl+c` | Quit |
| `Esc` | Close popup / go back |

### Sidebar

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `l` / `Enter` | Select model and load records |

### Main panel

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `h` | Focus sidebar |
| `r` | Refresh records |
| `n` | New record (if `create_endpoint` configured) |
| `e` / `Enter` | Edit selected record (if `update_endpoint` configured) |
| `d` | Delete selected record (if `delete_endpoint` configured) |
| `L` | Next page (if `pagination` configured on the model) |
| `H` | Previous page (served from cache, never re-fetches) |

### Create / Edit form

Form fields support vim-style modal editing. Forms open in **Insert** mode, so
typing works immediately if you never touch the vim motions.

| Key | Mode | Action |
|-----|------|--------|
| `Tab` / `Shift+Tab` | either | Next / previous field |
| `Ctrl+s` | either | Submit form |
| `Esc` | Insert | Switch to Normal mode |
| `Esc` | Normal | Cancel and close the form |
| `i` / `a` | Normal | Insert before / after cursor |
| `I` / `A` | Normal | Insert at line start / end |
| `o` | Normal | Open a new line below and insert |
| `h` `j` `k` `l` / arrows | Normal | Move cursor left/down/up/right |
| `w` / `b` | Normal | Jump to next / previous word |
| `0` / `$` | Normal | Jump to line start / end |
| `x` | Normal | Delete char under cursor |
| `dd` | Normal | Delete current line |
| `Enter` | Insert | Insert newline |
| `Backspace` | Insert | Delete char before cursor |

### Delete confirm

| Key | Action |
|-----|--------|
| `y` | Confirm delete |
| `n` / `Esc` | Cancel |

---

## Config reference

### `config.yaml`

| Field | Type | Description |
|-------|------|-------------|
| `base_url` | string | Base URL prepended to all API paths |
| `headers` | map | HTTP headers sent with every request |

Values support `${ENV_VAR}` substitution from `.env`.

### `models.yaml`

| Field | Required | Description |
|-------|----------|-------------|
| `name` | yes | Display name shown in the sidebar |
| `endpoint` | yes | Path for listing records (GET) |
| `create_endpoint` | no | Path for creating a record (POST) |
| `update_endpoint` | no | Path for updating a record (PUT). Use `{id}` as a placeholder. |
| `delete_endpoint` | no | Path for deleting a record (DELETE). Use `{id}` as a placeholder. |
| `id_field` | yes | JSON field used as the record identifier |
| `display_field` | yes | JSON field shown as the record label in the list |
| `fields` | no | Ordered list of fields shown in create/edit forms. Auto-detected from records if omitted. |
| `pagination` | no | Enables paginated fetching for this model's `endpoint`. See below. |

#### `pagination` (optional, per model)

Omit entirely for endpoints that return every record in one response (today's
default behavior). When present, tag the block with a `style`:

| Style | Extra fields | Use for |
|-------|--------------|---------|
| `page` | `page_param` (default `page`), `size_param` (default `per_page`), `page_size`, `first_page` (default `1`), `total_pages_field`, `has_more_field` | `?page=1&per_page=50`-style APIs |
| `offset` | `offset_param` (default `offset`), `limit_param` (default `limit`), `limit` | `?offset=0&limit=100`-style APIs |
| `cursor` | `cursor_param` (default `cursor`), `next_cursor_field` | APIs returning a next-page token in the response body |

`total_pages_field`/`has_more_field`/`next_cursor_field` are dot-paths into the
JSON response (e.g. `meta.next_cursor`). For `page` style, if both
`total_pages_field` and `has_more_field` are omitted, pagination stops once a
page returns fewer than `page_size` records. See `models.example.yaml` for
worked examples of all three styles.

---

## License

MIT OR Apache-2.0

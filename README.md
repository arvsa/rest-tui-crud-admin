# plop-tui

A terminal UI dashboard for generic CRUD operations against any REST API — think `lazygit` but for REST resources. Fully config-driven: no code changes needed to work with a new API.

Built with [ratatui](https://github.com/ratatui-org/ratatui) + Tokio in Rust.

---

## Features

- Sidebar lists all models defined in `models.yaml`
- Main panel fetches and displays records in a table
- Create, edit, and delete records (each action is optional per model)
- Multi-line text-area input with automatic overflow wrapping
- Help overlay (`?`) with keybinding reference
- Config-driven: swap `config.yaml` + `models.yaml` to target any REST API
- Sensitive values (tokens, URLs) stay in `.env` — never hard-coded

---

## Installation

Requires [Rust](https://rustup.rs/) (stable).

```bash
git clone https://github.com/your-username/plop-tui
cd plop-tui
cargo build --release
# binary is at ./target/release/plop-tui
```

---

## Setup

**1. Create your `.env`**

```bash
cp .env.example .env
```

Edit `.env` and fill in your API credentials:

```env
PLOP_BASE_URL=https://api.example.com
PLOP_AUTH_TOKEN=your-token-here
PLOP_BYPASS_TOKEN=your-bypass-token-here   # optional
```

**2. Create `config.yaml`**

```bash
cp config.example.yaml config.yaml
```

`config.yaml` sets the base URL and HTTP headers sent with every request.
Reference `.env` values with `${VAR_NAME}`:

```yaml
base_url: "${PLOP_BASE_URL}"
headers:
  Content-Type: "application/json"
  Authorization: "Bearer ${PLOP_AUTH_TOKEN}"
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
./target/release/plop-tui
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

### Create / Edit form

| Key | Action |
|-----|--------|
| `Tab` | Next field |
| `Shift+Tab` | Previous field |
| `Enter` | Insert newline (text-area mode) |
| `Ctrl+s` | Submit form |
| `Esc` | Cancel and close |

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

---

## License

MIT OR Apache-2.0

# Mastermind — Rust backend

A small Axum + Tokio service that stores Mastermind problems in memory and
serves the static SPA at the project root.

## Run

```bash
cd backend
cargo run
```

By default the server listens on `http://0.0.0.0:3000` and serves:

- **`GET  /`** — the SPA (`../index.html`, `../styles.css`, …).
- **`/api/*`** — JSON endpoints (see below).

Override with environment variables:

| Var                     | Default       | Meaning                                                         |
| ----------------------- | ------------- | --------------------------------------------------------------- |
| `PORT`                  | `3000`        | TCP port to listen on                                           |
| `MASTERMIND_STATIC_DIR` | `..`          | Directory served at `/` (the SPA)                               |
| `MASTERMIND_DB_PATH`    | XDG data home | SQLite database file path (see Storage section)                 |
| `RUST_LOG`              | `info`        | tracing filter                                                  |

Open `http://localhost:3000/` in a browser. The masthead's status dot will
flip from "Connecting…" (grey) to **"Live backend"** (green) once the SPA
detects the real API.

If you instead open the SPA standalone (e.g. via a static file server with no
backend), the status flips to **"Local mock"** (amber) and all data is
persisted in `localStorage` — fine for trying the UI, not shareable.

## API

All bodies are JSON. Colours are `u8` indices into the palette
(`0` = Crimson, `1` = Mustard, … `5` = Ink).

### `POST /api/problems` — create a problem

```json
{
  "code":        [0, 3, 5, 1],
  "settings":    { "codeLength": 4, "numColors": 6, "allowDuplicates": true, "maxAttempts": 10 },
  "initialGuesses": [[2, 2, 2, 2], [1, 1, 0, 0]],
  "title":       "Monday warm-up"
}
```

Response (`200 OK`): the **public** view of the problem (no `code`).

```json
{
  "id":              "k3m7q9z",
  "settings":        { ... },
  "initialGuesses":  [[2,2,2,2],[1,1,0,0]],
  "initialFeedback": [{"blacks":0,"whites":0},{"blacks":1,"whites":1}],
  "createdAt":       "2026-05-27T10:32:01Z",
  "title":           "Monday warm-up"
}
```

Errors: `400` on validation (bad colour, length mismatch, duplicates when
forbidden, an initial guess that already solves the code, title longer than
80 chars, or puzzle informazione-teoricamente irrisolvibile nel caso peggiore).

### `GET /api/problems` — list

```json
{ "problems": [ { /* public view */ }, … ] }
```

### `GET /api/problems/:id` — fetch one

Public view (no `code`). `404` if unknown.

### `POST /api/problems/:id/check` — evaluate a guess

```json
{ "guess": [0, 1, 2, 3] }
```

Response:

```json
{ "blacks": 2, "whites": 1 }
```

The server is stateless about player progress — it just scores guesses
against the stored code. The frontend tracks attempts.

### `PATCH /api/problems/:id` — rename a puzzle

Only the `title` is editable; `code`, `settings`, and `initialGuesses` are
frozen at creation to avoid breaking solvers already mid-game.

```json
{ "title": "Pasqua 2026" }
```

- `title` is trimmed; empty or whitespace-only becomes `null`.
- Max length 80 characters (enforced both at request layer and via DB CHECK).
- `404` if id is unknown.

Response (`200 OK`): the public view of the puzzle with the updated title.

### `DELETE /api/problems/:id` — delete a puzzle

Hard delete, no soft-delete or recovery.

- `204 No Content` on success.
- `404` if id is unknown.

### `GET /api/problems/:id/code` — reveal the code

```json
{ "code": [0, 3, 5, 1] }
```

The frontend only calls this once a game is won or lost. To make it
unfair-by-construction, you can:
- issue a session token at problem-load time and require it here, or
- count guesses server-side and refuse reveals until the budget is spent.

Both are intentionally left out — they're an exercise depending on how
much you care about cheaters in an anonymous, link-shared game.

## Storage

Puzzles are persisted in a local SQLite file (`rusqlite` with `bundled`
SQLite, no system dependency). Concurrency: a single shared
`Arc<Mutex<Connection>>` — sufficient for LAN-scale traffic.

**Default path** (in order of fallback):

1. `MASTERMIND_DB_PATH` env var, if set
2. `${XDG_DATA_HOME}/mastermind/mastermind.db`
3. `${HOME}/.local/share/mastermind/mastermind.db`
4. `./mastermind.db` (last resort, logged as a warning)

The parent directory is created automatically. WAL journal is enabled at
boot. Schema initialisation is idempotent (`CREATE TABLE IF NOT EXISTS`),
no migration crate needed at this scale.

**Backup**: just copy the `.db` file (with the binary stopped or via
`sqlite3 mastermind.db ".backup '/tmp/mastermind.bak'"`).

**Reset**: stop the binary and `rm mastermind.db*` (also deletes `-wal`/`-shm`
files). The next start recreates a fresh schema.

## Build

```bash
cargo build --release
./target/release/mastermind-backend
```

## Tests

```bash
cargo test
```

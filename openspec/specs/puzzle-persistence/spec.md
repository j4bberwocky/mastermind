# puzzle-persistence Specification

## Purpose
TBD - created by archiving change puzzle-lifecycle. Update Purpose after archive.
## Requirements
### Requirement: Puzzle store SHALL survive backend restarts

The backend SHALL persist all created puzzles to a SQLite database file on disk so that they remain available across process restarts, host reboots, and crashes.

#### Scenario: Puzzle created before restart is still listed after restart

- **WHEN** a client creates a puzzle via `POST /api/problems`, the server shuts down, and the server is restarted
- **THEN** the puzzle SHALL appear in the response of `GET /api/problems` after restart with all original fields preserved (id, code, settings, initial guesses, initial feedback, title, createdAt)

#### Scenario: Multiple puzzles persist correctly

- **WHEN** 5 puzzles are created and the backend is restarted
- **THEN** all 5 SHALL be retrievable via `GET /api/problems/:id` for each id, returning the same content as before restart

### Requirement: Database path SHALL be configurable via environment variable

The backend SHALL read the database file path from `MASTERMIND_DB_PATH` if set. When unset, it SHALL default to `${XDG_DATA_HOME:-$HOME/.local/share}/mastermind/mastermind.db`. The parent directory SHALL be created automatically if it does not exist.

#### Scenario: Explicit path via env var

- **WHEN** the backend is started with `MASTERMIND_DB_PATH=/tmp/test.db`
- **THEN** the file `/tmp/test.db` SHALL be created (if not present) and used as the store

#### Scenario: Default path with XDG_DATA_HOME set

- **WHEN** the backend is started with `XDG_DATA_HOME=/custom/data` and `MASTERMIND_DB_PATH` unset
- **THEN** the database SHALL be created at `/custom/data/mastermind/mastermind.db`

#### Scenario: Default path without XDG_DATA_HOME

- **WHEN** the backend is started with both `MASTERMIND_DB_PATH` and `XDG_DATA_HOME` unset and `HOME=/home/pi`
- **THEN** the database SHALL be created at `/home/pi/.local/share/mastermind/mastermind.db`

#### Scenario: Missing parent directory

- **WHEN** the configured DB path's parent directory does not exist
- **THEN** the backend SHALL create the parent directory (recursive) before opening the database

### Requirement: Schema SHALL be initialized idempotently on startup

The backend SHALL execute `CREATE TABLE IF NOT EXISTS` (and `CREATE INDEX IF NOT EXISTS`) at startup to ensure the schema exists, without destroying or modifying any existing data.

#### Scenario: First run creates schema

- **WHEN** the backend starts against a non-existent database file
- **THEN** a new SQLite database SHALL be created with the `problems` table and `idx_problems_created_at` index

#### Scenario: Subsequent runs preserve data

- **WHEN** the backend starts against an existing database with N puzzles
- **THEN** all N puzzles SHALL remain in the database after schema initialization completes

### Requirement: Listing SHALL be deterministically ordered

`GET /api/problems` SHALL return puzzles ordered by `createdAt DESC, id` so that two puzzles created in the same second appear in a stable, reproducible order across requests.

#### Scenario: Two puzzles same timestamp

- **WHEN** two puzzles are created within the same second with ids `abc1234` and `xyz5678`
- **THEN** `GET /api/problems` SHALL return them in an order that is identical across consecutive requests

### Requirement: Concurrent CRUD operations SHALL be serialized safely

The backend SHALL serialize all database operations through a single shared `Mutex<rusqlite::Connection>` so that concurrent requests from multiple clients do not corrupt the store nor produce torn reads.

#### Scenario: Concurrent create from 5 clients

- **WHEN** 5 clients issue `POST /api/problems` simultaneously
- **THEN** all 5 puzzles SHALL be persisted with unique ids and retrievable via subsequent `GET` requests

### Requirement: Backend SHALL refuse to start on unrecoverable storage errors

If the configured database path cannot be opened (e.g., permission denied, disk full, corrupted file), the backend SHALL log a clear error message identifying the path and exit with a non-zero status, rather than starting in a degraded state.

#### Scenario: Read-only filesystem

- **WHEN** `MASTERMIND_DB_PATH` points to a path on a read-only filesystem
- **THEN** the backend SHALL log an error containing the path and exit with a non-zero status code


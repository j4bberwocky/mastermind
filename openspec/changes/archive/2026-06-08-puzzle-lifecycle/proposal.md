## Why

Lo store dei puzzle è oggi un `Arc<RwLock<HashMap<String, Problem>>>` in RAM ([backend/src/main.rs:96-99](../../../backend/src/main.rs#L96-L99)): ogni riavvio del backend cancella tutto il lavoro. Per il target del progetto (Raspberry Pi nella LAN famigliare) questo significa che ogni aggiornamento di sistema, blackout, o reboot fa sparire i puzzle che il genitore ha composto per i figli. Manca inoltre la possibilità di rinominare o cancellare un puzzle pubblicato, e si possono creare puzzle informazione-teoricamente irrisolvibili (es. 4 pegs / 6 colori / 2 tentativi ⇒ 1296 candidati, servono ≥ 3 mosse).

## What Changes

- Sostituire lo store in-memory con un database SQLite locale persistente. Default path `${XDG_DATA_HOME:-$HOME/.local/share}/mastermind/mastermind.db`, override via `MASTERMIND_DB_PATH`.
- Aggiungere `PATCH /api/problems/:id` per modificare il `title` di un puzzle esistente (solo il titolo: code, settings e initial guesses restano immutabili dopo la pubblicazione per non rompere partite in corso).
- Aggiungere `DELETE /api/problems/:id` per cancellare un puzzle.
- Aggiungere nella BrowsePage frontend i controlli di rinomina inline e cestino con modal di conferma (i tap accidentali su tablet sono troppo facili senza conferma).
- Aggiungere un gate di validità informazione-teorico in `POST /api/problems`: il puzzle è accettato sse `R ≥ ⌈log_P(N)⌉` dove `N` è il numero di codici consistenti con le initial guesses, `P = (L+1)(L+2)/2 - 1` è il numero di pattern di feedback distinti per codeLength `L`, e `R` sono i tentativi rimasti. Se il check fallisce → 400 con messaggio "Puzzle non risolvibile nel caso peggiore: servono almeno {min} tentativi, ne hai {R}".
- Sostituire `std::sync::RwLock` con `Arc<Mutex<rusqlite::Connection>>` (cleanup gratuito di un TODO "Correttezza").
- **BREAKING (in pratica no)**: il binario richiede ora accesso in scrittura a `MASTERMIND_DB_PATH` o alla directory data XDG. Per chi gira il binario dal repo serve un mkdir, nessun impatto su API o sul mock localStorage frontend.

## Capabilities

### New Capabilities

- `puzzle-persistence`: i puzzle creati sopravvivono ai riavvii del backend tramite uno store SQLite locale; copre layout dello schema, lifecycle della connessione, percorso del file su disco, e contratti di durabilità delle operazioni CRUD.
- `puzzle-management`: rinomina (solo `title`) e cancellazione di puzzle esistenti tramite API `PATCH /api/problems/:id` e `DELETE /api/problems/:id`, più i corrispettivi controlli UI nella BrowsePage con conferma su delete.
- `puzzle-validity`: rifiuto server-side di puzzle informazione-teoricamente irrisolvibili nel caso peggiore, calcolato per enumerazione esatta dei candidati consistenti con le initial guesses.

### Modified Capabilities

Nessuna — questo è il primo change spec-driven del progetto, non ci sono ancora capacità documentate da modificare.

## Impact

**Codice toccato:**
- [backend/Cargo.toml](../../../backend/Cargo.toml): nuova dipendenza `rusqlite` (feature `bundled`).
- [backend/src/main.rs](../../../backend/src/main.rs): refactor `AppState`, nuovi moduli (estratti come file separati o submodule inline a scelta in implementazione), nuovi handler `update_problem` e `delete_problem`, integrazione validity check in `create_problem`, refactor di `list_problems`/`get_problem`/`check_guess`/`reveal_code` per usare lo store SQLite.
- [index.html](../../../index.html): nuovi metodi `api.updateProblem` e `api.deleteProblem`, controlli rinomina e cestino nella `BrowsePage`, modal di conferma.

**API esterne:**
- Nuovi endpoint `PATCH /api/problems/:id` e `DELETE /api/problems/:id`.
- Errore 400 aggiuntivo da `POST /api/problems` con messaggio di feasibility.
- Tutti gli altri endpoint conservano la stessa interfaccia.

**Dipendenze:**
- Aggiunta `rusqlite = { version = "...", features = ["bundled"] }`.
- Nessuna libreria di migrations (init schema inline al boot).

**Sistemi:**
- Il binario richiede ora accesso filesystem in scrittura al path del DB. Per il deploy Pi questo significa scegliere il path nello unit systemd (ad esempio `WorkingDirectory=/var/lib/mastermind` + `MASTERMIND_DB_PATH=/var/lib/mastermind/mastermind.db`).

**Risolve in passing (TODO già aperti nel README):**
- Sostituzione `std::sync::RwLock` async.
- Ordinamento stabile della list problems (diventa `ORDER BY created_at DESC, id`).
- Validazione lunghezza titolo server-side (`CHECK` constraint nel DB + validator Rust).
- Rimozione `ListResponse<'a>` dead code.

**Out of scope (restano TODO separati):**
- Build statica frontend / vendoring React.
- Cross-compile aarch64, systemd unit, mDNS via avahi.
- Cambio default `MASTERMIND_STATIC_DIR` da `..` a `./static`.
- Re-probe backend lato frontend, dep array `useEffect`, bug swap drag-and-drop desktop.
- Mock `localStorage` frontend: resta read-only per i campi nuovi (no edit/delete in modalità mock).

## 1. Setup dipendenze

- [x] 1.1 Aggiungere `rusqlite = { version = "0.31", features = ["bundled"] }` a [backend/Cargo.toml](../../../backend/Cargo.toml)
- [x] 1.2 Verificare che `cargo build --release` compili senza richiedere librerie SQLite di sistema (la feature `bundled` deve compilare SQLite statico)
- [x] 1.3 Aggiungere `rusqlite` ai dev-dependencies se servono helper di test diversi (probabilmente no — la stessa crate basta)

## 2. Layer di storage

- [x] 2.1 Creare modulo `storage` (file `backend/src/storage.rs` o submodule inline in `main.rs`) con tipo `Db` che incapsula `Arc<Mutex<rusqlite::Connection>>`
- [x] 2.2 Implementare `Db::open(path: &Path) -> Result<Self, StorageError>` che crea la directory parent se manca, apre la connessione, esegue `PRAGMA journal_mode = WAL`, ed esegue `init_schema`
- [x] 2.3 Implementare `init_schema(&Connection)` che esegue `CREATE TABLE IF NOT EXISTS problems(...)` con tutti i campi e CHECK constraints definiti in [design.md](design.md) §D3, e `CREATE INDEX IF NOT EXISTS idx_problems_created_at ON problems(created_at DESC, id)`
- [x] 2.4 Implementare `Db::insert(&self, problem: &Problem) -> Result<(), StorageError>` (serializza `initial_guesses` / `initial_feedback` come JSON, `code` come BLOB, `allow_duplicates` come 0/1)
- [x] 2.5 Implementare `Db::get(&self, id: &str) -> Result<Option<Problem>, StorageError>` con deserializzazione inversa
- [x] 2.6 Implementare `Db::list(&self) -> Result<Vec<Problem>, StorageError>` con `ORDER BY created_at DESC, id`
- [x] 2.7 Implementare `Db::update_title(&self, id: &str, title: Option<&str>) -> Result<bool, StorageError>` (ritorna `true` se ha aggiornato una riga, `false` se id non esiste, e aggiorna `updated_at`)
- [x] 2.8 Implementare `Db::delete(&self, id: &str) -> Result<bool, StorageError>` (ritorna `true` se ha cancellato una riga, `false` se id non esiste)
- [x] 2.9 Definire `StorageError` enum (variants per `Io`, `Sqlite`, `Serde`); implementare `From` verso `AppError` esistente

## 3. Refactor AppState e handler esistenti

- [x] 3.1 Sostituire `AppState { problems: Arc<RwLock<HashMap<...>>> }` con `AppState { db: Db }` (`Db` è internamente già `Arc`-clone-friendly)
- [x] 3.2 Aggiornare `create_problem` per chiamare `state.db.insert(&problem)` invece di `HashMap::insert`
- [x] 3.3 Aggiornare `list_problems` per chiamare `state.db.list()` e rimuovere il sort manuale (oggi è `b.created_at.cmp(&a.created_at)` su `Vec<&Problem>`)
- [x] 3.4 Aggiornare `get_problem` per chiamare `state.db.get(&id)`
- [x] 3.5 Aggiornare `check_guess` per `state.db.get(&id)` (legge solo, niente write)
- [x] 3.6 Aggiornare `reveal_code` per `state.db.get(&id)`
- [x] 3.7 Rimuovere `ListResponse<'a>` (dead code segnalato nel README) e tutto codice di sort ora inutile
- [x] 3.8 Verificare che tutti gli `.unwrap()` sui lock siano spariti (erano la causa del TODO "panic poisons lock")

## 4. Config path del DB

- [x] 4.1 In `main()`, calcolare il path del DB: `std::env::var("MASTERMIND_DB_PATH").unwrap_or_else(|_| default_db_path())`
- [x] 4.2 Implementare `default_db_path()` che ritorna `${XDG_DATA_HOME:-$HOME/.local/share}/mastermind/mastermind.db`; usare `std::env::var` per `XDG_DATA_HOME` e `HOME`; fallback (entrambi mancanti) a `./mastermind.db` con warning log
- [x] 4.3 Loggare il path effettivo al boot insieme alla riga `listening on http://...`
- [x] 4.4 In caso di errore di apertura DB: log esplicito con path, `std::process::exit(1)`

## 5. Endpoint PATCH

- [x] 5.1 Definire struct `PatchRequest { title: Option<String> }` con `#[serde(default)]` per accettare anche body vuoto
- [x] 5.2 Implementare handler `update_problem(State(state), Path(id), Json(req)) -> Result<Json<Problem>, AppError>` che: estrae title se presente, trimma whitespace, normalizza a `None` se empty/whitespace, valida lunghezza ≤ 80 (altrimenti `bad("titolo troppo lungo...")`); chiama `state.db.update_title(&id, normalized.as_deref())`; se ritorna `false` → `not_found()`; altrimenti rilegge il problem con `db.get` e ritorna public view
- [x] 5.3 Registrare la route: `.route("/problems/:id", get(get_problem).patch(update_problem).delete(delete_problem))`

## 6. Endpoint DELETE

- [x] 6.1 Implementare handler `delete_problem(State(state), Path(id)) -> Result<StatusCode, AppError>`: chiama `state.db.delete(&id)`; se `false` → `not_found()`; altrimenti ritorna `StatusCode::NO_CONTENT`

## 7. Validity gate

- [x] 7.1 Creare modulo `validity` (file `backend/src/validity.rs` o submodule inline)
- [x] 7.2 Implementare `pattern_count(code_length: usize) -> usize` con formula `(L+1)*(L+2)/2 - 1`
- [x] 7.3 Implementare `min_attempts_needed(candidates: usize, code_length: usize) -> usize` come `((candidates as f64).log(pattern_count(L) as f64).ceil() as usize).max(1)` con cura per `candidates == 1` (richiede 0 ma diciamo 1 per uniformità, già coperto dal check "initial guess solves")
- [x] 7.4 Implementare iteratore `domain(settings: &Settings) -> impl Iterator<Item = Vec<u8>>`: se `allow_duplicates`, prodotto cartesiano `C^L`; altrimenti permutazioni `P(C, L)` (usare un iteratore custom o `itertools::permutations`; valutare se aggiungere `itertools` o scrivere a mano — preferenza: a mano per evitare dep)
- [x] 7.5 Implementare `count_candidates(settings: &Settings, initial_guesses: &[Vec<u8>], initial_feedback: &[Feedback]) -> usize`: itera `domain(settings)`, per ogni candidate verifica che per ogni `i` si abbia `evaluate_guess(&initial_guesses[i], &candidate) == initial_feedback[i]`, accumula
- [x] 7.6 Implementare `validate_solvable(settings: &Settings, initial_guesses: &[Vec<u8>], initial_feedback: &[Feedback]) -> Result<(), String>`: calcola `N`, `R`, `min_needed`; se `R < min_needed` ritorna `Err(format!("Puzzle non risolvibile nel caso peggiore: servono almeno {min_needed} tentativi, ne hai {R}"))`; altrimenti `Ok(())`
- [x] 7.7 Integrare `validate_solvable` in `create_problem` DOPO il check "initial guess already solves" e PRIMA dell'insert; in caso di errore restituire `bad(msg)`

## 8. Frontend API client

- [x] 8.1 In [index.html](../../../index.html), aggiungere `api.updateProblem(id, patch)`: `PATCH /api/problems/:id` con body `JSON.stringify(patch)`; gestire 200 (ritorna json), 400 (throw con message), 404 (throw con `e.code = 404`)
- [x] 8.2 Aggiungere `api.deleteProblem(id)`: `DELETE /api/problems/:id`; gestire 204 (ritorna void), 404 (throw con `e.code = 404`)
- [x] 8.3 Per entrambi: se siamo in modalità mock, throw immediato con un errore tipo "operazione non supportata in modalità locale" (verrà nascosto dalla UI comunque, ma defense-in-depth)

## 9. Frontend BrowsePage controls

- [x] 9.1 In `BrowsePage`, recuperare `mode` da `api.mode` per condizionare la UI
- [x] 9.2 Aggiungere stato locale `editing: { id, value } | null` e `confirmingDelete: id | null`
- [x] 9.3 Quando `mode === "remote"`: su ogni `.problem-card` aggiungere icona matita (rinomina) e cestino (cancella) come elementi non-anchor (per non triggerare il link)
- [x] 9.4 Implementare inline edit: click su matita → input al posto del titolo, focus, Enter conferma (chiama `api.updateProblem`), Esc annulla, blur senza modifiche annulla
- [x] 9.5 Implementare modal di conferma delete: contenitore overlay full-screen, due bottoni, Esc/backdrop click annullano, click Cancella chiama `api.deleteProblem`
- [x] 9.6 Su successo update: aggiornare l'item nello stato locale `items` senza ri-fetchare la lista
- [x] 9.7 Su successo delete: rimuovere l'item dallo stato locale
- [x] 9.8 Su 404 (per qualunque motivo): rimuovere l'item dalla lista e mostrare un toast/notice "Puzzle non trovato"
- [x] 9.9 Quando `mode !== "remote"`: NON mostrare matita né cestino (vedi spec puzzle-management)

## 10. Frontend CreatePage error display

- [x] 10.1 Quando `api.createProblem` fallisce con un errore di validità (status 400 con messaggio "Puzzle non risolvibile..."), assicurarsi che il messaggio venga mostrato chiaramente nel componente di pubblicazione (oggi è già il caso, verificare il rendering del messaggio italiano)

## 11. CSS per nuovi controlli

- [x] 11.1 Aggiungere a [styles.css](../../../styles.css) stili per `.problem-card-actions` (contenitore icone), `.icon-btn` (matita/cestino), `.confirm-modal`/`.confirm-modal-backdrop`, e l'input inline di rinomina
- [x] 11.2 Verificare che i target tap siano ≥ 44×44 px su tablet (vincolo Apple HIG / Material)

## 12. Tests backend

- [x] 12.1 Test unitari per `validity::pattern_count` (casi L=2..8 confrontati con valori calcolati a mano)
- [x] 12.2 Test unitari per `validity::count_candidates`:
  - 4×6, no initial → 1296
  - 4×6 no-dup, no initial → 360
  - 4×6 con 1 initial guess + feedback noto → conteggio atteso (calcolato manualmente o per riferimento incrociato)
- [x] 12.3 Test `validity::validate_solvable`:
  - 4×6 maxAttempts=10 no initial → Ok
  - 4×6 maxAttempts=2 no initial → Err con stringa che contiene "almeno 3"
  - 8×8 dup maxAttempts=4 → Err
- [x] 12.4 Test integrazione `storage`: aprire un DB `:memory:`, insert/get/list/update_title/delete; verificare round-trip identità su tutti i campi (specialmente `code` BLOB e arrays JSON)
- [x] 12.5 Test integrazione handler: usare `axum::Router` in-process con `Db` su `:memory:`; verificare POST/GET/PATCH/DELETE end-to-end
- [x] 12.6 Test che POST con title > 80 char restituisce 400
- [x] 12.7 Test che PATCH su id sconosciuto restituisce 404; DELETE idem

## 13. Documentazione

- [x] 13.1 Aggiornare [backend/README.md](../../../backend/README.md): aggiungere PATCH e DELETE alla sezione API; aggiungere sezione "Storage" che spiega path/env var; rimuovere la frase "il store è in memoria" e sostituire con descrizione SQLite
- [x] 13.2 Aggiornare [README.md](../../../README.md): nella sezione "Limitazioni note" rimuovere "Niente persistenza"; nella sezione "TODO" marcare come ✅ (o cancellare) le voci: persistenza, RwLock async, ordinamento stabile, validazione lunghezza titolo, ListResponse, aggiungere riga su nuove env var (`MASTERMIND_DB_PATH`)
- [x] 13.3 Aggiungere alla tabella env vars del README la variabile `MASTERMIND_DB_PATH`

## 14. Verifica manuale end-to-end

- [x] 14.1 `cargo run`, creare 3 puzzle, riavviare il binario, verificare che siano ancora lì
- [x] 14.2 Provare a creare un puzzle "impossibile" (4×6 maxAttempts=2): verificare che venga rifiutato con messaggio chiaro
- [x] 14.3 Aprire la BrowsePage, rinominare un puzzle inline, ricaricare la pagina, verificare persistenza
- [x] 14.4 Cancellare un puzzle con conferma, verificare che sparisca da list e che `GET /api/problems/:id` torni 404
- [x] 14.5 Aprire la SPA senza backend (es. via `file://`): verificare che matita/cestino siano assenti (mock mode)
- [x] 14.6 Su un tablet reale (o devtools touch emulation): verificare che il modal di conferma sia tappabile senza problemi e che non si attivi al primo tap sul cestino

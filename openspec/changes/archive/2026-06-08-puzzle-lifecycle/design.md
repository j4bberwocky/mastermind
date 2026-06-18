## Context

Mastermind è un toy project: backend Rust monolitico (Axum) + SPA React monofile servita dallo stesso binario, store oggi in `Arc<RwLock<HashMap<String, Problem>>>` in [backend/src/main.rs:96-99](../../../backend/src/main.rs#L96-L99). Target di deploy: un Raspberry Pi 4/5 nella LAN famigliare, 3-5 tablet, ≤ 300 puzzle attesi nell'arco di vita del sistema.

Le tre sotto-capacità di questa change condividono lo stesso layer di storage, quindi vivono in una change unica per evitare due refactor consecutivi dello stesso codice.

Vincoli rilevanti:
- Cross-compile su `aarch64-unknown-linux-gnu` deve restare semplice (no toolchain di sistema oltre `cross`).
- Niente daemon DB esterno (no Postgres). Tutto in-process.
- Niente auth — la sicurezza viene dal fatto che il servizio è raggiungibile solo dalla LAN.
- Latenza target sub-100ms anche su Pi.

## Goals / Non-Goals

**Goals:**

- I puzzle pubblicati sopravvivono a riavvii del backend.
- L'autore di un puzzle può rinominarlo o cancellarlo dalla UI di archivio.
- Puzzle informazione-teoricamente irrisolvibili nel caso peggiore vengono rifiutati alla creazione con messaggio comprensibile.
- Concorrenza sicura (5 tablet che chiamano in contemporanea) senza pool di connessioni.
- Cross-compile per ARM64 non richiede pacchetti di sistema sul Pi né sulla macchina di build.

**Non-Goals:**

- Edit di `code`, `settings`, `initial_guesses`, `allow_duplicates`: esplicitamente immutabili dopo create.
- Soft-delete / cestino con recupero: la cancellazione è hard.
- Migration framework, schema versioning sofisticato: a 300 righe non serve.
- Backup automatico: l'utente può copiare il file `.db` con scp.
- Frontend mock localStorage: edit/delete/validity solo in modalità "Live backend".
- Indicatore di difficoltà UI: scope concordato è "solo blocco impossibile".
- Endpoint `/validate` separato per pre-check: lo stesso check viene eseguito su POST /problems e l'errore restituito al client.
- Auth/CSRF/rate-limit: contesto LAN famigliare.

## Decisions

### D1. `rusqlite` (feature `bundled`) invece di `sqlx`

`rusqlite` è sync e si sposa con i nostri handler Axum senza bisogno di `tokio::task::spawn_blocking` per le query brevi tipiche di questo dominio (insert/select su una sola tabella). `sqlx` richiederebbe `DATABASE_URL` al compile-time per i query macros e introdurrebbe un'API async dove non serve.

La feature `bundled` compila SQLite statico nel binario: zero dipendenze native sul Pi, cross-compile via `cross` resta lineare.

**Alternative considerate:**
- `sqlx`: type-safety delle query a compile time è un bel-to-have, ma costa ergonomia significativa.
- `sled`/`redb`: ottimi DB Rust-native ma con file format meno portabile e meno conosciuti. Per la nostra scala SQLite vince per familiarità.

### D2. `Arc<Mutex<rusqlite::Connection>>` singolo, no pool

5 tablet in famiglia non saturano nemmeno una connessione SQLite. Un `Mutex` singolo è più semplice di un pool (es. `r2d2`) e SQLite è comunque single-writer internamente.

Gli handler prendono il lock, eseguono la query, rilasciano. Tempi di query attesi: <1ms per SELECT puntuali, <5ms per INSERT con WAL.

**Trade-off:** SELECT concorrenti vengono serializzati. A 5 client non si percepisce.

### D3. Schema denormalizzato, JSON inline per `initial_guesses` / `initial_feedback`

A ≤ 300 righe e zero query analitiche (no aggregati, no filter per peg color), normalizzare in tabelle figlie `problem_guesses(problem_id, idx, row, blacks, whites)` è puro overhead. Le colonne JSON sono lette/scritte in blocco insieme al record.

```sql
CREATE TABLE IF NOT EXISTS problems (
  id                  TEXT PRIMARY KEY,
  code                BLOB NOT NULL,
  code_length         INTEGER NOT NULL,
  num_colors          INTEGER NOT NULL,
  allow_duplicates    INTEGER NOT NULL,
  max_attempts        INTEGER NOT NULL,
  initial_guesses     TEXT NOT NULL DEFAULT '[]',
  initial_feedback    TEXT NOT NULL DEFAULT '[]',
  title               TEXT CHECK(title IS NULL OR length(title) <= 80),
  created_at          TEXT NOT NULL,
  updated_at          TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_problems_created_at
  ON problems(created_at DESC, id);
```

L'indice secondario `(created_at DESC, id)` rende deterministico l'ordinamento della list (`ORDER BY created_at DESC, id`), risolvendo gratis il TODO "ordinamento stabile".

### D4. Schema init inline, no crate di migrations

`CREATE TABLE IF NOT EXISTS` al boot, fine. Se in futuro lo schema evolve si aggiunge una `PRAGMA user_version` + match Rust con piccole `ALTER TABLE`. A questa scala non vale la pena adottare `refinery` o simili.

### D5. Default path: `${XDG_DATA_HOME:-$HOME/.local/share}/mastermind/mastermind.db`

Convenzione XDG su Linux/macOS. Su Pi i path tipici diventano `/home/pi/.local/share/mastermind/mastermind.db` per esecuzione utente o `/var/lib/mastermind/mastermind.db` se gestito da systemd come servizio dedicato (override esplicito via env).

La directory parent viene creata al boot se non esiste (`std::fs::create_dir_all`).

**Variabile**: `MASTERMIND_DB_PATH` ha precedenza sul default. Path relativo accettato (risolto rispetto a `cwd`).

### D6. Title-only edit

`code`, `settings`, `initial_guesses` sono semanticamente "frozen" dopo la pubblicazione: cambiarli a metà di una partita di un solver romperebbe l'esperienza (initial_feedback inconsistente, code lunghezza errata, eccetera). Per cambi profondi il workflow è **delete + create nuovo**.

`title` invece è puramente cosmetico: editabile senza conseguenze. Trim, vuoto/whitespace-only → NULL, max 80 char.

**Alternative considerate:**
- Edit completo finché nessuno ha giocato: richiederebbe tracciare i tentativi server-side. Complica troppo lo store senza ROI in famiglia.
- `maxAttempts` editabile (solo aumento): ammissibile, ma il caso d'uso è speculativo e aggiunge superficie di validazione. Skip.

### D7. Validità: enumerazione esatta, no euristiche

La regola informazione-teorica `R ≥ ⌈log_P(N)⌉` è una condizione necessaria di risolvibilità nel caso peggiore (un solver ottimo può sempre dimezzare-per-pattern lo spazio).

`N` = numero di codici nel dominio consistenti con le feedback delle initial guesses. Calcolato per enumerazione esaustiva:

```text
domain(L, C, allow_dup):
  if allow_dup: prodotto cartesiano C^L
  else:         permutazioni senza ripetizioni P(C, L)

count_candidates(initial_guesses, initial_feedback, settings):
  count = 0
  for code in domain(settings):
    if for_all i: evaluate(initial_guesses[i], code) == initial_feedback[i]:
      count += 1
  return count
```

Worst case: L=8, C=8, allow_dup → `8^8 = 16.7M` codici. Loop in Rust release con `evaluate_guess` esistente: stimati ~50ms su CPU desktop, ~150-200ms su Pi 4. Accettabile per `POST /problems` (one-shot, no spam).

**P** è calcolato in forma chiusa: `(L+1)(L+2)/2 - 1` (coppie (b,w) con `b + w ≤ L` meno la coppia degenere `(L-1, 1)` che è impossibile).

**Alternative considerate:**
- Knuth-style worst-case depth analysis: NP-hard in generale, soluzioni note solo per casi specifici (5 per 4×6).
- Heuristic: rifiutare solo quando "ovviamente" troppo poco (es. R < L). Sotto-restrittivo: 4×6 maxAttempts=4 verrebbe accettato ma può fallire.

L'enumerazione è la scelta corretta: esatta, abbastanza veloce, codice semplice.

### D8. Modal di conferma sul delete frontend

Senza modal, su tablet un tap accidentale sul cestino cancella istantaneamente il puzzle. Con bambini che usano i tablet questo è inaccettabile. Modal con due bottoni "Cancella" / "Annulla", "Cancella" come azione distruttiva con stile rosso.

Rinomina invece non ha conferma: l'edit-in-place ha già un "annulla" naturale (tasto Esc / focus loss).

### D9. AppError esistente come canale errori

Il pattern `AppError(StatusCode, String)` di [backend/src/main.rs:208-224](../../../backend/src/main.rs#L208-L224) gestisce già 400/404/500. Estendiamo l'enum con varianti se serve (es. `ValidationError` con payload strutturato), ma per la prima iterazione tornare 400 con stringa è coerente con il resto del codice.

## Risks / Trade-offs

- **[Disk full / DB corruption]** → Il backend va in panic all'avvio se non riesce a creare/aprire il DB. Log esplicito su stderr che indica il path. Mitigazione: l'admin (l'utente sul Pi) interviene manualmente. Non c'è auto-recovery.

- **[Lock contention sotto traffico burst]** → 5 tablet che spammano `/check` durante la stessa partita serializzano sul Mutex. Test approssimato: 5 client × 1 req/s × 5ms = 25ms/s carico. Non è un problema reale. Se lo diventasse, sostituire il Mutex con `parking_lot::Mutex` (più veloce) o passare a un pool.

- **[Validity check lento al worst case]** → 8×8×allow_dup richiede ~150ms su Pi. POST /problems blocca per quel tempo. Mitigazione: per i casi realistici (default 4×6) il check vale <1ms; il caso worst-case è raro e accettabile. Se diventasse fastidioso si può spostare il check in `tokio::task::spawn_blocking`.

- **[Migrazione dati esistenti]** → Non c'è alcun dato esistente da migrare: l'in-memory store è effimero. Primo `cargo run` con la nuova versione crea uno schema vergine, vita facile.

- **[Frontend mock divergence]** → Il mock localStorage non implementa edit/delete/validity. Utenti che aprono la SPA in modalità mock vedranno bottoni della UI (rinomina/cestino) che falliscono o sono nascosti. Mitigazione: nascondere i controlli nuovi quando `api.mode === "mock"`. Documentare in README.

- **[XDG fallback su Windows]** → `$HOME/.local/share` non è la convenzione corretta su Windows. Per il target Pi/macOS non importa, ma se in futuro qualcuno builda su Windows il default sarà subottimale. Mitigazione: documentare di settare `MASTERMIND_DB_PATH` esplicitamente.

- **[Concorrenza creazione titolo duplicato]** → Due puzzle con lo stesso titolo sono ammessi (no constraint). Coerente con il comportamento attuale. Non è un problema, solo una nota.

## Migration Plan

Nessun dato esistente da migrare. Steps:

1. Bumpare versione binario.
2. Sul Pi: scegliere il path del DB (suggerisco `/home/pi/.local/share/mastermind/mastermind.db` o `/var/lib/mastermind/mastermind.db`), assicurarsi che la directory parent esista (`mkdir -p`).
3. Avviare il nuovo binario. Lo schema viene creato vuoto al primo avvio.
4. Per chi gira il binario dal repo: nessuna azione, la directory XDG è creata automaticamente nel home dell'utente.

**Rollback:** non distruttivo. Tornare alla versione precedente del binario: lo store in-memory ricomincia da zero, il file `.db` resta intatto su disco. Non è "un rollback dei dati" ma "un rollback del codice".

## Open Questions

1. **Devo nascondere o disabilitare i controlli rinomina/cestino quando il frontend è in modalità mock?** Tendo a nasconderli (cleaner UI in mock), ma un disable con tooltip "richiede backend live" è più educativo. Decisione da prendere in implementazione.

2. **Validare anche `code_length` / `num_colors` rispetto al DB CHECK?** Oggi `validate_settings` ha range 2..=8. Posso replicare i constraint nel DB (`CHECK(code_length BETWEEN 2 AND 8)`) per defense in depth, o lasciare la responsabilità solo al codice Rust. Lean verso replicare nel DB: costa zero e protegge da bug futuri.

3. **Faccio `PRAGMA journal_mode = WAL` al boot?** WAL migliora drasticamente le scritture concorrenti SQLite. A questa scala non cambia nulla percettibile, ma è una best-practice gratuita. Probabilmente sì.

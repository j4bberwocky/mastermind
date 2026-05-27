# Mastermind

Un Mastermind giocabile dal browser, pensato per una rete locale di famiglia
(Raspberry Pi nella LAN, tablet dei bambini come client).

## Tipologia di progetto

Applicazione web full-stack in due parti, distribuita come **singolo binario**:

- **Backend Rust** in [backend/](backend/) — [Axum](https://docs.rs/axum) + Tokio,
  store in-memory (`Arc<RwLock<HashMap>>`). Espone un'API JSON sotto `/api/*` e
  serve la SPA come file statici sotto `/`.
- **Frontend SPA** alla radice del repo ([index.html](index.html),
  [styles.css](styles.css)). React 18 + JSX trasformato da Babel-standalone
  **a runtime nel browser** (nessuna build step richiesta — vedi sezione
  "Limitazioni note"). Router via hash (`#/`, `#/create`, `#/play/:id`,
  `#/browse`).

Un singolo `cargo run` avvia tutto: il binario serve sia la SPA sia l'API
sullo stesso processo, stessa origine, niente CORS da configurare.

L'API è documentata in [backend/README.md](backend/README.md).

## Prerequisiti

- [Rust toolchain](https://rustup.rs/) (edition 2021, testato con stable).
- Un browser moderno con connessione a internet **al primo caricamento** —
  React/Babel sono caricati da `unpkg.com` via CDN.

## Build & run (sviluppo)

```bash
cd backend
cargo run
```

Per default ascolta su `http://0.0.0.0:3000` e serve la SPA dalla directory
padre (`..`). Aprire `http://localhost:3000` nel browser.

Variabili d'ambiente disponibili:

| Var                     | Default | Significato                 |
| ----------------------- | ------- | --------------------------- |
| `PORT`                  | `3000`  | Porta TCP                   |
| `MASTERMIND_STATIC_DIR` | `..`    | Directory servita sotto `/` |
| `RUST_LOG`              | `info`  | Filtro `tracing`            |

## Build di produzione

```bash
cd backend
cargo build --release
./target/release/mastermind-backend
```

Il binario risultante è autosufficiente. Per deployarlo basta copiare:

- `backend/target/release/mastermind-backend`
- `index.html` e `styles.css` (in una directory raggiungibile via
  `MASTERMIND_STATIC_DIR`)

## Test

```bash
cd backend
cargo test
```

Coprono la logica di `evaluate_guess` (blacks/whites) e la generazione degli
ID. Niente test end-to-end al momento.

## Limitazioni note

- **Babel a runtime nel browser**: ~2 MB di JS scaricati e trasformati a ogni
  caricamento. Va bene in LAN, ma una build statica con esbuild/Vite è in
  programma per ridurre il tempo di avvio sui tablet meno recenti.
- **Niente persistenza**: lo store è in RAM, si svuota al riavvio del backend.
- **Niente autenticazione**: l'endpoint `GET /api/problems/:id/code` rivela
  il codice a chiunque conosca l'ID. Accettabile in LAN famigliare, non in
  pubblico.
- **Touch**: drag-and-drop disabilitato su dispositivi touch; l'interazione è
  tap-to-place. Drag rimane su desktop.

## TODO

### Deploy Raspberry Pi (obiettivo principale)

- [ ] Build statica del frontend (esbuild/Vite) — vendorare React/ReactDOM in
      `static/`, eliminare Babel runtime e le CDN `unpkg.com`. Senza questo,
      se il router di casa è offline la webapp non si avvia.
- [ ] Persistenza su disco: scrivere lo store su `~/.local/share/mastermind/db.json`
      a ogni mutazione, oppure passare a `rusqlite` con SQLite bundled.
- [ ] Cross-compile per `aarch64-unknown-linux-gnu` (Pi 4/5) da macOS via
      [`cross`](https://github.com/cross-rs/cross).
- [ ] Unit `systemd` per auto-start al boot e restart on crash.
- [ ] mDNS via `avahi-daemon` (preinstallato su Raspberry Pi OS) per accedere
      a `http://mastermind.local:3000/` invece dell'IP.
- [ ] Cambiare default di `MASTERMIND_STATIC_DIR` da `..` a `./static` —
      la directory padre espone codice sorgente quando il binario gira dal repo.

### Correttezza

- [ ] Sostituire `std::sync::RwLock` con `tokio::sync::RwLock` o
      `parking_lot::RwLock` — quello attuale blocca un worker Tokio sotto
      contesa, e un panic in un handler avvelena il lock per tutti gli altri.
- [ ] Sistemare lo "swap" tra due slot in `handleDropToSlot` ([index.html](index.html))
      che sovrascrive il target senza liberare la sorgente.
- [ ] Ordinamento stabile della lista problemi (oggi è ISO-8601 stringa: due
      problemi creati nello stesso secondo escono in ordine non deterministico).
- [ ] Validare lunghezza titolo lato server (frontend cappa a 80, backend
      accetta qualsiasi cosa).
- [ ] Aggiungere array di dipendenze al `useEffect` dei keyboard shortcut
      ([index.html](index.html), `<Game>`) — oggi si ri-binda ad ogni render.
- [ ] Re-probe del backend lato frontend: oggi se il backend torna su dopo
      essere stato giù la SPA resta in "Local mock" fino al refresh.

### Pulizia

- [ ] Rimuovere `ListResponse<'a>` mai usato in `backend/src/main.rs`.
- [ ] Committare `backend/Cargo.lock` (per un binary crate va versionato).
- [ ] Aggiungere un `.gitignore` alla root del repo (oggi `target/` non è
      ignorata fuori da `backend/`).
- [ ] Verifica reale del touch su iPad/Android — finora testato solo via
      DevTools.

### Nice-to-have

- [ ] LICENSE (era presente prima della riscrittura, è stato rimosso).
- [ ] Dockerfile + `docker-compose.yml` opzionali se in futuro si vuole
      deployare in container invece che binario nudo.
- [ ] Test end-to-end via `reqwest` o Playwright.

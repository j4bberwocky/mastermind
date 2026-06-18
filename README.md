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

- [Rust toolchain](https://rustup.rs/) (edition 2021). `rustup` userà
  automaticamente il canale e i componenti definiti in
  [`backend/rust-toolchain.toml`](backend/rust-toolchain.toml) entrando
  nella cartella.
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

| Var                     | Default       | Significato                                     |
| ----------------------- | ------------- | ----------------------------------------------- |
| `PORT`                  | `3000`        | Porta TCP                                       |
| `MASTERMIND_STATIC_DIR` | `..`          | Directory servita sotto `/`                     |
| `MASTERMIND_DB_PATH`    | XDG data home | Path file SQLite (vedi `backend/README.md`)     |
| `RUST_LOG`              | `info`        | Filtro `tracing`                                |

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
- [x] ~~Persistenza su disco~~ — fatto in `puzzle-lifecycle`: SQLite via
      `rusqlite` bundled, path da `MASTERMIND_DB_PATH` o XDG data home.
- [ ] Cross-compile per `aarch64-unknown-linux-gnu` (Pi 4/5) da macOS via
      [`cross`](https://github.com/cross-rs/cross).
- [ ] Unit `systemd` per auto-start al boot e restart on crash.
- [ ] mDNS via `avahi-daemon` (preinstallato su Raspberry Pi OS) per accedere
      a `http://mastermind.local:3000/` invece dell'IP.
- [ ] Cambiare default di `MASTERMIND_STATIC_DIR` da `..` a `./static` —
      la directory padre espone codice sorgente quando il binario gira dal repo.

### Correttezza

- [x] ~~Sostituire `std::sync::RwLock`~~ — risolto in `puzzle-lifecycle`:
      sostituito da `Arc<Mutex<rusqlite::Connection>>` (SQLite gestisce
      internamente la concorrenza, niente lock poisoning sui worker Tokio).
- [ ] Sistemare lo "swap" tra due slot in `handleDropToSlot` ([index.html](index.html))
      che sovrascrive il target senza liberare la sorgente.
- [x] ~~Ordinamento stabile della lista problemi~~ — risolto in
      `puzzle-lifecycle`: ora `ORDER BY created_at DESC, id` lato SQL.
- [x] ~~Validare lunghezza titolo lato server~~ — risolto in `puzzle-lifecycle`:
      max 80 caratteri, validato in `create_problem` e `update_problem`,
      più `CHECK` constraint nel DB come defense in depth.
- [ ] Aggiungere array di dipendenze al `useEffect` dei keyboard shortcut
      ([index.html](index.html), `<Game>`) — oggi si ri-binda ad ogni render.
- [ ] Re-probe del backend lato frontend: oggi se il backend torna su dopo
      essere stato giù la SPA resta in "Local mock" fino al refresh.

### Pulizia

- [x] ~~Rimuovere `ListResponse<'a>`~~ — eliminato in `puzzle-lifecycle`.
- [x] ~~Committare `backend/Cargo.lock`~~ — `backend/.gitignore` aggiornato
      per non ignorare il lockfile (binary crate, build riproducibili).
- [x] ~~Aggiungere un `.gitignore` alla root del repo~~ — file `.gitignore`
      root presente, ignora `**/target/` e `.DS_Store`.
- [ ] Verifica reale del touch su iPad/Android — finora testato solo via
      DevTools.

### Nice-to-have

- [ ] LICENSE (era presente prima della riscrittura, è stato rimosso).
- [ ] Dockerfile + `docker-compose.yml` opzionali se in futuro si vuole
      deployare in container invece che binario nudo.
- [ ] Test end-to-end via `reqwest` o Playwright.

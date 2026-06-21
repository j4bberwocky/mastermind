# Makefile per mastermind — wrapper sui comandi cargo del backend Rust.
# La toolchain è gestita da rustup (backend/rust-toolchain.toml), NON da mise.
# I comandi girano in backend/ (CARGO_DIR); il binario serve anche la SPA dalla
# radice del repo, quindi `make run` si lancia da qui.
#
# Esempi:
#   make run        # cargo run (dev): API + SPA su http://localhost:3000
#   make build      # cargo build --release (binario autosufficiente)
#   make test       # cargo test
#   make fmt        # cargo fmt
#   make fmt-check  # cargo fmt --check (gate CI)
#   make clippy     # cargo clippy -- -D warnings
#   make clean      # cargo clean
#   make            # = fmt-check clippy test (gate qualità)

CARGO     ?= cargo
CARGO_DIR ?= backend

.PHONY: all run build test fmt fmt-check clippy clean check help

all: check fmt-check clippy test

run: check
	cd $(CARGO_DIR) && $(CARGO) run

build: check
	cd $(CARGO_DIR) && $(CARGO) build --release

test: check
	cd $(CARGO_DIR) && $(CARGO) test

fmt: check
	cd $(CARGO_DIR) && $(CARGO) fmt

fmt-check: check
	cd $(CARGO_DIR) && $(CARGO) fmt --check

clippy: check
	cd $(CARGO_DIR) && $(CARGO) clippy --all-targets -- -D warnings

clean: check
	cd $(CARGO_DIR) && $(CARGO) clean

check:
	@command -v $(CARGO) >/dev/null 2>&1 || { echo "ERRORE: '$(CARGO)' non trovato. Rust è gestito da rustup: installa da https://rustup.rs (userà backend/rust-toolchain.toml)"; exit 1; }

help:
	@sed -n '1,/^$$/p' $(MAKEFILE_LIST) | sed 's/^# \{0,1\}//'

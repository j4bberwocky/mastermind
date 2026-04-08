# ── Stage 1: Builder ────────────────────────────────────────────────────────
FROM rust:1.81-slim AS builder

WORKDIR /app

# Cache dependencies by copying manifests first
COPY Cargo.toml Cargo.lock ./
# Create a dummy main so Cargo can resolve + fetch deps
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo fetch

# Now copy the real source and build
COPY src ./src
RUN cargo build --release

# ── Stage 2: Runtime ─────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

# Install minimal runtime deps (TLS, etc.)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the compiled binary
COPY --from=builder /app/target/release/mastermind ./mastermind

# Copy the static frontend assets
COPY static ./static

# Expose the application port
EXPOSE 8080

# Run as a non-root user for security
RUN useradd -m appuser && chown -R appuser:appuser /app
USER appuser

ENV RUST_LOG=info

ENTRYPOINT ["./mastermind"]

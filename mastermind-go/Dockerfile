# ── Stage 1: Builder ────────────────────────────────────────────────────────
FROM golang:1.22-alpine AS builder

WORKDIR /app

# Cache dependencies by copying go.mod/go.sum first
COPY go.mod go.sum ./
RUN go mod download

# Copy source and build
COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -o mastermind ./cmd/mastermind

# ── Stage 2: Runtime ─────────────────────────────────────────────────────────
FROM alpine:3.19 AS runtime

# Install minimal runtime deps
RUN apk add --no-cache ca-certificates curl

WORKDIR /app

# Copy the compiled binary
COPY --from=builder /app/mastermind ./mastermind

# Copy the static frontend assets
COPY static ./static

# Expose the application port
EXPOSE 8080

# Run as a non-root user for security
RUN adduser -D appuser && chown -R appuser:appuser /app
USER appuser

ENV LOG_LEVEL=info

ENTRYPOINT ["./mastermind"]

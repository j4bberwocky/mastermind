.PHONY: build test lint fmt run docker clean

# Build the binary
build:
	go build -o bin/mastermind ./cmd/mastermind

# Run all tests with race detector
test:
	go test -race -v ./...

# Run linter (requires golangci-lint)
lint:
	golangci-lint run

# Format code
fmt:
	gofmt -w .

# Run the server locally
run:
	go run ./cmd/mastermind

# Build and run with Docker Compose
docker:
	docker compose up --build

# Clean build artifacts
clean:
	rm -rf bin/

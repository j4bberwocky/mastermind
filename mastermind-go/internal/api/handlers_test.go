package api

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/go-chi/chi/v5"

	"github.com/j4bberwocky/mastermind/mastermind-go/internal/game"
	"github.com/j4bberwocky/mastermind/mastermind-go/internal/state"
)

// setupRouter creates a test router with all API routes.
func setupRouter() (*Handler, *chi.Mux) {
	s := state.New()
	h := NewHandler(s)
	r := chi.NewRouter()
	r.Get("/api/health", h.Health)
	r.Post("/api/game", h.CreateGame)
	r.Get("/api/game/{id}", h.GetGame)
	r.Post("/api/game/{id}/guess", h.MakeGuess)
	r.Post("/api/game/{id}/analyze", h.Analyze)
	r.Get("/api/game/{id}/export", h.ExportGame)
	return h, r
}

func TestHealthEndpoint(t *testing.T) {
	_, r := setupRouter()

	req := httptest.NewRequest(http.MethodGet, "/api/health", nil)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("expected 200, got %d", w.Code)
	}

	var resp map[string]string
	json.NewDecoder(w.Body).Decode(&resp)
	if resp["status"] != "ok" {
		t.Errorf("expected status ok, got %s", resp["status"])
	}
	if resp["service"] != "mastermind" {
		t.Errorf("expected service mastermind, got %s", resp["service"])
	}
}

func TestCreateGame(t *testing.T) {
	_, r := setupRouter()

	req := httptest.NewRequest(http.MethodPost, "/api/game", nil)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusCreated {
		t.Errorf("expected 201, got %d", w.Code)
	}

	var resp NewGameResponse
	json.NewDecoder(w.Body).Decode(&resp)
	if resp.GameID == "" {
		t.Error("expected non-empty game_id")
	}
	if resp.CodeLength != game.CodeLength {
		t.Errorf("expected code_length %d, got %d", game.CodeLength, resp.CodeLength)
	}
	if resp.NumColors != game.NumColors {
		t.Errorf("expected num_colors %d, got %d", game.NumColors, resp.NumColors)
	}
	if resp.MaxAttempts != game.MaxAttempts {
		t.Errorf("expected max_attempts %d, got %d", game.MaxAttempts, resp.MaxAttempts)
	}
}

func TestGetGameNotFound(t *testing.T) {
	_, r := setupRouter()

	req := httptest.NewRequest(http.MethodGet, "/api/game/nonexistent", nil)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusNotFound {
		t.Errorf("expected 404, got %d", w.Code)
	}
}

func TestGetGameInProgress(t *testing.T) {
	h, r := setupRouter()

	g := game.NewGameWithSecret("test-1", game.Code{1, 2, 3, 4})
	h.State.Set("test-1", g)

	req := httptest.NewRequest(http.MethodGet, "/api/game/test-1", nil)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("expected 200, got %d", w.Code)
	}

	var resp GameStateResponse
	json.NewDecoder(w.Body).Decode(&resp)
	if resp.Secret != nil {
		t.Error("expected nil secret while in progress")
	}
	if resp.Status != game.StatusInProgress {
		t.Errorf("expected in_progress, got %s", resp.Status)
	}
}

func TestMakeGuessHappyPath(t *testing.T) {
	h, r := setupRouter()

	g := game.NewGameWithSecret("test-1", game.Code{1, 2, 3, 4})
	h.State.Set("test-1", g)

	body, _ := json.Marshal(GuessRequest{Guess: []uint8{1, 1, 2, 2}})
	req := httptest.NewRequest(http.MethodPost, "/api/game/test-1/guess", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("expected 200, got %d", w.Code)
	}

	var resp GuessResponse
	json.NewDecoder(w.Body).Decode(&resp)
	if resp.Attempt != 1 {
		t.Errorf("expected attempt 1, got %d", resp.Attempt)
	}
	if resp.Status != game.StatusInProgress {
		t.Errorf("expected in_progress, got %s", resp.Status)
	}
}

func TestMakeGuessInvalidCode(t *testing.T) {
	h, r := setupRouter()

	g := game.NewGameWithSecret("test-1", game.Code{1, 2, 3, 4})
	h.State.Set("test-1", g)

	// Wrong length
	body, _ := json.Marshal(GuessRequest{Guess: []uint8{1, 2, 3}})
	req := httptest.NewRequest(http.MethodPost, "/api/game/test-1/guess", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected 400, got %d", w.Code)
	}
}

func TestMakeGuessGameOver(t *testing.T) {
	h, r := setupRouter()

	g := game.NewGameWithSecret("test-1", game.Code{1, 2, 3, 4})
	// Win the game first
	g.Guess(game.Code{1, 2, 3, 4})
	h.State.Set("test-1", g)

	body, _ := json.Marshal(GuessRequest{Guess: []uint8{1, 2, 3, 4}})
	req := httptest.NewRequest(http.MethodPost, "/api/game/test-1/guess", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusConflict {
		t.Errorf("expected 409, got %d", w.Code)
	}
}

func TestMakeGuessWin(t *testing.T) {
	h, r := setupRouter()

	g := game.NewGameWithSecret("test-1", game.Code{1, 2, 3, 4})
	h.State.Set("test-1", g)

	body, _ := json.Marshal(GuessRequest{Guess: []uint8{1, 2, 3, 4}})
	req := httptest.NewRequest(http.MethodPost, "/api/game/test-1/guess", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("expected 200, got %d", w.Code)
	}

	var resp GuessResponse
	json.NewDecoder(w.Body).Decode(&resp)
	if resp.Status != game.StatusWon {
		t.Errorf("expected won, got %s", resp.Status)
	}
	if resp.Secret == nil {
		t.Error("expected secret to be revealed on win")
	}
	if resp.Blacks != 4 {
		t.Errorf("expected 4 blacks, got %d", resp.Blacks)
	}
}

func TestMakeGuessNotFound(t *testing.T) {
	_, r := setupRouter()

	body, _ := json.Marshal(GuessRequest{Guess: []uint8{1, 2, 3, 4}})
	req := httptest.NewRequest(http.MethodPost, "/api/game/nonexistent/guess", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusNotFound {
		t.Errorf("expected 404, got %d", w.Code)
	}
}

func TestAnalyzeInProgress(t *testing.T) {
	h, r := setupRouter()

	g := game.NewGameWithSecret("test-1", game.Code{1, 2, 3, 4})
	h.State.Set("test-1", g)

	req := httptest.NewRequest(http.MethodPost, "/api/game/test-1/analyze", nil)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusConflict {
		t.Errorf("expected 409, got %d", w.Code)
	}
}

func TestAnalyzeCompletedGame(t *testing.T) {
	h, r := setupRouter()

	g := game.NewGameWithSecret("test-1", game.Code{1, 2, 3, 4})
	g.Guess(game.Code{1, 1, 2, 2})
	g.Guess(game.Code{1, 2, 3, 4})
	h.State.Set("test-1", g)

	req := httptest.NewRequest(http.MethodPost, "/api/game/test-1/analyze", nil)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("expected 200, got %d", w.Code)
	}

	var resp map[string]interface{}
	json.NewDecoder(w.Body).Decode(&resp)
	if resp["game_id"] != "test-1" {
		t.Errorf("expected game_id test-1, got %v", resp["game_id"])
	}
	if resp["status"] != "won" {
		t.Errorf("expected won status, got %v", resp["status"])
	}
}

func TestExportGame(t *testing.T) {
	h, r := setupRouter()

	g := game.NewGameWithSecret("test-1", game.Code{1, 2, 3, 4})
	g.Guess(game.Code{1, 2, 3, 4})
	h.State.Set("test-1", g)

	req := httptest.NewRequest(http.MethodGet, "/api/game/test-1/export", nil)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("expected 200, got %d", w.Code)
	}

	var resp GameExport
	json.NewDecoder(w.Body).Decode(&resp)
	if resp.SchemaVersion != "1.0" {
		t.Errorf("expected schema_version 1.0, got %s", resp.SchemaVersion)
	}
	if resp.GameID != "test-1" {
		t.Errorf("expected game_id test-1, got %s", resp.GameID)
	}
	if resp.Variant.CodeLength != game.CodeLength {
		t.Errorf("expected code_length %d, got %d", game.CodeLength, resp.Variant.CodeLength)
	}
	if len(resp.Turns) != 1 {
		t.Errorf("expected 1 turn, got %d", len(resp.Turns))
	}
	if resp.Status != game.StatusWon {
		t.Errorf("expected won, got %s", resp.Status)
	}
}

func TestExportGameNotFound(t *testing.T) {
	_, r := setupRouter()

	req := httptest.NewRequest(http.MethodGet, "/api/game/nonexistent/export", nil)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusNotFound {
		t.Errorf("expected 404, got %d", w.Code)
	}
}

func TestFullGameFlow(t *testing.T) {
	h, r := setupRouter()

	// Create game with known secret
	g := game.NewGameWithSecret("flow-test", game.Code{3, 4, 5, 6})
	h.State.Set("flow-test", g)

	// Submit a wrong guess
	body, _ := json.Marshal(GuessRequest{Guess: []uint8{1, 2, 3, 4}})
	req := httptest.NewRequest(http.MethodPost, "/api/game/flow-test/guess", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}

	var guessResp GuessResponse
	json.NewDecoder(w.Body).Decode(&guessResp)
	if guessResp.Status != game.StatusInProgress {
		t.Errorf("expected in_progress, got %s", guessResp.Status)
	}

	// Submit the correct guess
	body, _ = json.Marshal(GuessRequest{Guess: []uint8{3, 4, 5, 6}})
	req = httptest.NewRequest(http.MethodPost, "/api/game/flow-test/guess", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	w = httptest.NewRecorder()
	r.ServeHTTP(w, req)

	json.NewDecoder(w.Body).Decode(&guessResp)
	if guessResp.Status != game.StatusWon {
		t.Errorf("expected won, got %s", guessResp.Status)
	}

	// Get game state
	req = httptest.NewRequest(http.MethodGet, "/api/game/flow-test", nil)
	w = httptest.NewRecorder()
	r.ServeHTTP(w, req)

	var stateResp GameStateResponse
	json.NewDecoder(w.Body).Decode(&stateResp)
	if stateResp.AttemptsUsed != 2 {
		t.Errorf("expected 2 attempts, got %d", stateResp.AttemptsUsed)
	}
	if stateResp.Secret == nil {
		t.Error("expected secret to be revealed")
	}

	// Export
	req = httptest.NewRequest(http.MethodGet, "/api/game/flow-test/export", nil)
	w = httptest.NewRecorder()
	r.ServeHTTP(w, req)

	var exportResp GameExport
	json.NewDecoder(w.Body).Decode(&exportResp)
	if len(exportResp.Turns) != 2 {
		t.Errorf("expected 2 export turns, got %d", len(exportResp.Turns))
	}
}

func TestMakeGuessInvalidJSON(t *testing.T) {
	h, r := setupRouter()

	g := game.NewGameWithSecret("test-1", game.Code{1, 2, 3, 4})
	h.State.Set("test-1", g)

	req := httptest.NewRequest(http.MethodPost, "/api/game/test-1/guess", bytes.NewReader([]byte("not json")))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected 400, got %d", w.Code)
	}
}

func TestMakeGuessInvalidPegValue(t *testing.T) {
	h, r := setupRouter()

	g := game.NewGameWithSecret("test-1", game.Code{1, 2, 3, 4})
	h.State.Set("test-1", g)

	body, _ := json.Marshal(GuessRequest{Guess: []uint8{1, 2, 3, 7}})
	req := httptest.NewRequest(http.MethodPost, "/api/game/test-1/guess", bytes.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected 400, got %d", w.Code)
	}
}

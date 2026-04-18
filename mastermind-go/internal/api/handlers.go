package api

import (
	"encoding/json"
	"fmt"
	"log/slog"
	"net/http"
	"time"

	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"

	"github.com/j4bberwocky/mastermind/mastermind-go/internal/analysis"
	"github.com/j4bberwocky/mastermind/mastermind-go/internal/game"
	"github.com/j4bberwocky/mastermind/mastermind-go/internal/state"
)

// Handler holds the application state and provides HTTP handler methods.
type Handler struct {
	State *state.AppState
}

// NewHandler creates a new Handler with the given application state.
func NewHandler(s *state.AppState) *Handler {
	return &Handler{State: s}
}

// Health handles GET /api/health
func (h *Handler) Health(w http.ResponseWriter, r *http.Request) {
	writeJSON(w, http.StatusOK, map[string]string{
		"status":  "ok",
		"service": "mastermind",
	})
}

// CreateGame handles POST /api/game
func (h *Handler) CreateGame(w http.ResponseWriter, r *http.Request) {
	id := uuid.New().String()
	g := game.NewGame(id)
	h.State.Set(id, g)

	slog.Info("New game created", "game_id", id)

	writeJSON(w, http.StatusCreated, NewGameResponse{
		GameID:      id,
		CodeLength:  game.CodeLength,
		NumColors:   game.NumColors,
		MaxAttempts: game.MaxAttempts,
		Message: fmt.Sprintf(
			"New game started! Guess the %d-peg code using colors 1–%d. You have %d attempts.",
			game.CodeLength, game.NumColors, game.MaxAttempts,
		),
	})
}

// GetGame handles GET /api/game/{id}
func (h *Handler) GetGame(w http.ResponseWriter, r *http.Request) {
	id := chi.URLParam(r, "id")
	g, ok := h.State.Get(id)
	if !ok {
		writeError(w, http.StatusNotFound, "Game not found")
		return
	}

	turns := make([]TurnResponse, len(g.Turns))
	for i, t := range g.Turns {
		turns[i] = TurnResponse{
			Attempt: t.Attempt,
			Guess:   t.Guess,
			Blacks:  t.Feedback.Blacks,
			Whites:  t.Feedback.Whites,
		}
	}

	writeJSON(w, http.StatusOK, GameStateResponse{
		GameID:       g.ID,
		Status:       g.Status,
		AttemptsUsed: len(g.Turns),
		MaxAttempts:  game.MaxAttempts,
		Turns:        turns,
		Secret:       g.RevealedSecret(),
	})
}

// MakeGuess handles POST /api/game/{id}/guess
func (h *Handler) MakeGuess(w http.ResponseWriter, r *http.Request) {
	id := chi.URLParam(r, "id")

	var body GuessRequest
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
		writeError(w, http.StatusBadRequest, "Invalid JSON body")
		return
	}

	code, err := game.ParseCode(body.Guess)
	if err != nil {
		writeError(w, http.StatusBadRequest, err.Error())
		return
	}

	var feedback game.Feedback
	var status game.GameStatus
	var secret []uint8
	var attempt int
	var guessErr error

	found := h.State.WithGame(id, func(g *game.Game) {
		feedback, guessErr = g.Guess(code)
		if guessErr != nil {
			return
		}
		status = g.Status
		secret = g.RevealedSecret()
		attempt = len(g.Turns)
	})

	if !found {
		writeError(w, http.StatusNotFound, "Game not found")
		return
	}

	if guessErr != nil {
		writeError(w, http.StatusConflict, guessErr.Error())
		return
	}

	var message string
	switch status {
	case game.StatusWon:
		message = fmt.Sprintf(
			"🎉 Congratulations! You guessed the secret in %d attempt(s)!",
			attempt,
		)
	case game.StatusLost:
		message = fmt.Sprintf(
			"Game over! The secret was %v. Better luck next time!",
			secret,
		)
	case game.StatusInProgress:
		message = fmt.Sprintf(
			"%d black(s), %d white(s). %d attempt(s) remaining.",
			feedback.Blacks, feedback.Whites, game.MaxAttempts-attempt,
		)
	}

	slog.Info("Guess evaluated",
		"game_id", id,
		"attempt", attempt,
		"blacks", feedback.Blacks,
		"whites", feedback.Whites,
		"status", status,
	)

	writeJSON(w, http.StatusOK, GuessResponse{
		Attempt: attempt,
		Blacks:  feedback.Blacks,
		Whites:  feedback.Whites,
		Status:  status,
		Secret:  secret,
		Message: message,
	})
}

// Analyze handles POST /api/game/{id}/analyze
func (h *Handler) Analyze(w http.ResponseWriter, r *http.Request) {
	id := chi.URLParam(r, "id")
	g, ok := h.State.Get(id)
	if !ok {
		writeError(w, http.StatusNotFound, "Game not found")
		return
	}

	if g.Status == game.StatusInProgress {
		writeError(w, http.StatusConflict,
			"Game is still in progress — finish the game before requesting analysis.")
		return
	}

	result := analysis.AnalyzeGame(g)

	slog.Info("Game analyzed", "game_id", id, "score", result.OptimalityScore)

	writeJSON(w, http.StatusOK, result)
}

// ExportGame handles GET /api/game/{id}/export
func (h *Handler) ExportGame(w http.ResponseWriter, r *http.Request) {
	id := chi.URLParam(r, "id")
	g, ok := h.State.Get(id)
	if !ok {
		writeError(w, http.StatusNotFound, "Game not found")
		return
	}

	turns := make([]ExportTurn, len(g.Turns))
	for i, t := range g.Turns {
		turns[i] = ExportTurn{
			Attempt: t.Attempt,
			Guess:   t.Guess,
			Blacks:  t.Feedback.Blacks,
			Whites:  t.Feedback.Whites,
		}
	}

	var finishedAt *string
	if g.FinishedAt != nil {
		s := g.FinishedAt.Format(time.RFC3339)
		finishedAt = &s
	}

	export := GameExport{
		SchemaVersion: "1.0",
		GameID:        g.ID,
		Variant: GameVariant{
			CodeLength:      game.CodeLength,
			NumColors:       game.NumColors,
			MaxAttempts:     game.MaxAttempts,
			AllowRepetition: true,
		},
		Status:     g.Status,
		StartedAt:  g.StartedAt.Format(time.RFC3339),
		FinishedAt: finishedAt,
		Turns:      turns,
		Secret:     g.RevealedSecret(),
	}

	writeJSON(w, http.StatusOK, export)
}

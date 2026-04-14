// Package api provides HTTP handlers and types for the Mastermind REST API.
package api

import (
	"github.com/j4bberwocky/mastermind/mastermind-go/internal/game"
)

// ── Request types ────────────────────────────────────────────────────────────

// GuessRequest is the JSON body for a guess submission.
type GuessRequest struct {
	Guess []uint8 `json:"guess"`
}

// ── Response types ───────────────────────────────────────────────────────────

// NewGameResponse is returned when a new game is created.
type NewGameResponse struct {
	GameID      string `json:"game_id"`
	CodeLength  int    `json:"code_length"`
	NumColors   uint8  `json:"num_colors"`
	MaxAttempts int    `json:"max_attempts"`
	Message     string `json:"message"`
}

// GameStateResponse is the public view of the game state.
type GameStateResponse struct {
	GameID       string          `json:"game_id"`
	Status       game.GameStatus `json:"status"`
	AttemptsUsed int             `json:"attempts_used"`
	MaxAttempts  int             `json:"max_attempts"`
	Turns        []TurnResponse  `json:"turns"`
	Secret       []uint8         `json:"secret"`
}

// TurnResponse represents a single turn in the game state response.
type TurnResponse struct {
	Attempt int     `json:"attempt"`
	Guess   []uint8 `json:"guess"`
	Blacks  uint8   `json:"blacks"`
	Whites  uint8   `json:"whites"`
}

// GuessResponse is returned after each guess.
type GuessResponse struct {
	Attempt int             `json:"attempt"`
	Blacks  uint8           `json:"blacks"`
	Whites  uint8           `json:"whites"`
	Status  game.GameStatus `json:"status"`
	Secret  []uint8         `json:"secret"`
	Message string          `json:"message"`
}

// GameExport is the portable game export format.
type GameExport struct {
	SchemaVersion string          `json:"schema_version"`
	GameID        string          `json:"game_id"`
	Variant       GameVariant     `json:"variant"`
	Status        game.GameStatus `json:"status"`
	StartedAt     string          `json:"started_at"`
	FinishedAt    *string         `json:"finished_at"`
	Turns         []ExportTurn    `json:"turns"`
	Secret        []uint8         `json:"secret"`
}

// GameVariant describes the game variant for export.
type GameVariant struct {
	CodeLength      int   `json:"code_length"`
	NumColors       uint8 `json:"num_colors"`
	MaxAttempts     int   `json:"max_attempts"`
	AllowRepetition bool  `json:"allow_repetition"`
}

// ExportTurn represents a turn in the export format.
type ExportTurn struct {
	Attempt int     `json:"attempt"`
	Guess   []uint8 `json:"guess"`
	Blacks  uint8   `json:"blacks"`
	Whites  uint8   `json:"whites"`
}

// ErrorResponse is the JSON error format.
type ErrorResponse struct {
	Error string `json:"error"`
}

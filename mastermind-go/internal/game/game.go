// Package game implements the core Mastermind domain types and game logic.
package game

import (
	"encoding/json"
	"errors"
	"fmt"
	"math/rand/v2"
	"time"
)

// Game constants matching the classic Mastermind variant.
const (
	CodeLength  = 4
	NumColors   = 6
	MaxAttempts = 10
)

// Sentinel errors for game operations.
var (
	ErrGameOver         = errors.New("game is already over")
	ErrInvalidPeg       = errors.New("invalid peg value")
	ErrInvalidCodeLength = errors.New("invalid code length")
	ErrNotFound         = errors.New("game not found")
)

// Peg represents a single color peg (1–6).
type Peg uint8

// NewPeg creates a Peg after validating the value is in [1, NumColors].
func NewPeg(v uint8) (Peg, error) {
	if v < 1 || v > NumColors {
		return 0, fmt.Errorf("invalid peg value: %d (must be 1–%d): %w", v, NumColors, ErrInvalidPeg)
	}
	return Peg(v), nil
}

// Code is a fixed-size array of CodeLength pegs.
type Code [CodeLength]Peg

// Feedback is the result of comparing a guess against the secret code.
type Feedback struct {
	Blacks uint8 `json:"blacks"`
	Whites uint8 `json:"whites"`
}

// IsPerfect returns true when the guess perfectly matched the secret.
func (f Feedback) IsPerfect() bool {
	return f.Blacks == CodeLength
}

// Turn records a single guess and its feedback.
type Turn struct {
	Attempt  int      `json:"attempt"`
	Guess    []uint8  `json:"guess"`
	Feedback Feedback `json:"feedback"`
}

// GameStatus represents the current status of a game.
type GameStatus string

const (
	StatusInProgress GameStatus = "in_progress"
	StatusWon        GameStatus = "won"
	StatusLost       GameStatus = "lost"
)

// MarshalJSON produces snake_case JSON strings for GameStatus.
func (s GameStatus) MarshalJSON() ([]byte, error) {
	return json.Marshal(string(s))
}

// UnmarshalJSON parses snake_case JSON strings into GameStatus.
func (s *GameStatus) UnmarshalJSON(data []byte) error {
	var str string
	if err := json.Unmarshal(data, &str); err != nil {
		return err
	}
	switch str {
	case "in_progress":
		*s = StatusInProgress
	case "won":
		*s = StatusWon
	case "lost":
		*s = StatusLost
	default:
		return fmt.Errorf("unknown game status: %s", str)
	}
	return nil
}

// Game holds the full game state stored in memory per session.
type Game struct {
	ID         string     `json:"id"`
	Secret     Code       `json:"secret"`
	Turns      []Turn     `json:"turns"`
	Status     GameStatus `json:"status"`
	StartedAt  time.Time  `json:"started_at"`
	FinishedAt *time.Time `json:"finished_at,omitempty"`
}

// NewGame creates a new game with a randomly generated secret code.
func NewGame(id string) *Game {
	return &Game{
		ID:        id,
		Secret:    generateSecret(),
		Turns:     make([]Turn, 0),
		Status:    StatusInProgress,
		StartedAt: time.Now().UTC(),
	}
}

// NewGameWithSecret creates a new game with a specified secret (for testing).
func NewGameWithSecret(id string, secret Code) *Game {
	return &Game{
		ID:        id,
		Secret:    secret,
		Turns:     make([]Turn, 0),
		Status:    StatusInProgress,
		StartedAt: time.Now().UTC(),
	}
}

// generateSecret creates a random secret code.
func generateSecret() Code {
	var code Code
	for i := range code {
		code[i] = Peg(rand.IntN(NumColors) + 1)
	}
	return code
}

// Guess makes a guess, records it, and returns the feedback.
func (g *Game) Guess(code Code) (Feedback, error) {
	if g.Status != StatusInProgress {
		return Feedback{}, ErrGameOver
	}

	feedback := Evaluate(g.Secret, code)
	attempt := len(g.Turns) + 1

	guess := make([]uint8, CodeLength)
	for i, p := range code {
		guess[i] = uint8(p)
	}

	g.Turns = append(g.Turns, Turn{
		Attempt:  attempt,
		Guess:    guess,
		Feedback: feedback,
	})

	if feedback.IsPerfect() {
		g.Status = StatusWon
		now := time.Now().UTC()
		g.FinishedAt = &now
	} else if len(g.Turns) >= MaxAttempts {
		g.Status = StatusLost
		now := time.Now().UTC()
		g.FinishedAt = &now
	}

	return feedback, nil
}

// RevealedSecret returns the secret only if the game is over.
func (g *Game) RevealedSecret() []uint8 {
	if g.Status == StatusInProgress {
		return nil
	}
	secret := make([]uint8, CodeLength)
	for i, p := range g.Secret {
		secret[i] = uint8(p)
	}
	return secret
}

// Evaluate computes the black and white peg feedback for a guess against a secret.
func Evaluate(secret, guess Code) Feedback {
	var blacks uint8
	var secretRemaining [NumColors + 1]uint8
	var guessRemaining [NumColors + 1]uint8

	for i := 0; i < CodeLength; i++ {
		if secret[i] == guess[i] {
			blacks++
		} else {
			secretRemaining[secret[i]]++
			guessRemaining[guess[i]]++
		}
	}

	var whites uint8
	for c := uint8(1); c <= NumColors; c++ {
		if secretRemaining[c] < guessRemaining[c] {
			whites += secretRemaining[c]
		} else {
			whites += guessRemaining[c]
		}
	}

	return Feedback{Blacks: blacks, Whites: whites}
}

// ParseCode validates a slice of uint8 values and converts them into a Code.
func ParseCode(values []uint8) (Code, error) {
	if len(values) != CodeLength {
		return Code{}, fmt.Errorf("code must have exactly %d pegs, got %d: %w", CodeLength, len(values), ErrInvalidCodeLength)
	}
	var code Code
	for i, v := range values {
		p, err := NewPeg(v)
		if err != nil {
			return Code{}, err
		}
		code[i] = p
	}
	return code, nil
}

// CodeToSlice converts a Code to a []uint8 slice.
func CodeToSlice(code Code) []uint8 {
	result := make([]uint8, CodeLength)
	for i, p := range code {
		result[i] = uint8(p)
	}
	return result
}

package game

import (
	"encoding/json"
	"errors"
	"testing"
)

// Helper to create a Code from 4 uint8 values.
func code(a, b, c, d uint8) Code {
	return Code{Peg(a), Peg(b), Peg(c), Peg(d)}
}

func TestPerfectMatch(t *testing.T) {
	secret := code(1, 2, 3, 4)
	guess := code(1, 2, 3, 4)
	fb := Evaluate(secret, guess)
	if fb.Blacks != 4 {
		t.Errorf("expected 4 blacks, got %d", fb.Blacks)
	}
	if fb.Whites != 0 {
		t.Errorf("expected 0 whites, got %d", fb.Whites)
	}
	if !fb.IsPerfect() {
		t.Error("expected IsPerfect to be true")
	}
}

func TestNoMatch(t *testing.T) {
	secret := code(1, 1, 1, 1)
	guess := code(2, 2, 2, 2)
	fb := Evaluate(secret, guess)
	if fb.Blacks != 0 {
		t.Errorf("expected 0 blacks, got %d", fb.Blacks)
	}
	if fb.Whites != 0 {
		t.Errorf("expected 0 whites, got %d", fb.Whites)
	}
}

func TestAllWhites(t *testing.T) {
	secret := code(1, 2, 3, 4)
	guess := code(4, 3, 2, 1)
	fb := Evaluate(secret, guess)
	if fb.Blacks != 0 {
		t.Errorf("expected 0 blacks, got %d", fb.Blacks)
	}
	if fb.Whites != 4 {
		t.Errorf("expected 4 whites, got %d", fb.Whites)
	}
}

func TestMixed(t *testing.T) {
	secret := code(1, 2, 3, 4)
	guess := code(1, 3, 5, 6)
	fb := Evaluate(secret, guess)
	if fb.Blacks != 1 {
		t.Errorf("expected 1 black, got %d", fb.Blacks)
	}
	if fb.Whites != 1 {
		t.Errorf("expected 1 white, got %d", fb.Whites)
	}
}

func TestDuplicateHandling(t *testing.T) {
	// Classic edge-case: secret [1,1,2,2], guess [1,1,1,1]
	secret := code(1, 1, 2, 2)
	guess := code(1, 1, 1, 1)
	fb := Evaluate(secret, guess)
	if fb.Blacks != 2 {
		t.Errorf("expected 2 blacks, got %d", fb.Blacks)
	}
	if fb.Whites != 0 {
		t.Errorf("expected 0 whites, got %d", fb.Whites)
	}
}

func TestGameWon(t *testing.T) {
	g := NewGameWithSecret("test", code(1, 2, 3, 4))
	fb, err := g.Guess(code(1, 2, 3, 4))
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !fb.IsPerfect() {
		t.Error("expected perfect feedback")
	}
	if g.Status != StatusWon {
		t.Errorf("expected status Won, got %s", g.Status)
	}
	if g.FinishedAt == nil {
		t.Error("expected FinishedAt to be set")
	}
}

func TestGameLostAfterMaxAttempts(t *testing.T) {
	g := NewGameWithSecret("test", code(1, 2, 3, 4))
	for i := 0; i < MaxAttempts; i++ {
		_, _ = g.Guess(code(5, 5, 5, 5))
	}
	if g.Status != StatusLost {
		t.Errorf("expected status Lost, got %s", g.Status)
	}
}

func TestCannotGuessAfterGameOver(t *testing.T) {
	g := NewGameWithSecret("test", code(1, 2, 3, 4))
	_, _ = g.Guess(code(1, 2, 3, 4))
	_, err := g.Guess(code(1, 2, 3, 4))
	if !errors.Is(err, ErrGameOver) {
		t.Errorf("expected ErrGameOver, got %v", err)
	}
}

func TestInvalidPeg(t *testing.T) {
	_, err := NewPeg(0)
	if err == nil {
		t.Error("expected error for peg 0")
	}
	_, err = NewPeg(7)
	if err == nil {
		t.Error("expected error for peg 7")
	}
	_, err = NewPeg(1)
	if err != nil {
		t.Errorf("unexpected error for peg 1: %v", err)
	}
	_, err = NewPeg(6)
	if err != nil {
		t.Errorf("unexpected error for peg 6: %v", err)
	}
}

func TestParseCodeWrongLength(t *testing.T) {
	_, err := ParseCode([]uint8{1, 2, 3})
	if err == nil {
		t.Error("expected error for 3 pegs")
	}
	_, err = ParseCode([]uint8{1, 2, 3, 4, 5})
	if err == nil {
		t.Error("expected error for 5 pegs")
	}
}

func TestSecretRevealedOnlyWhenDone(t *testing.T) {
	g := NewGameWithSecret("test", code(1, 2, 3, 4))
	if secret := g.RevealedSecret(); secret != nil {
		t.Error("expected nil secret while in progress")
	}
	_, _ = g.Guess(code(1, 2, 3, 4))
	secret := g.RevealedSecret()
	if secret == nil {
		t.Fatal("expected secret to be revealed")
	}
	expected := []uint8{1, 2, 3, 4}
	for i, v := range expected {
		if secret[i] != v {
			t.Errorf("secret[%d] = %d, want %d", i, secret[i], v)
		}
	}
}

// Table-driven tests for Evaluate with additional edge cases.
func TestEvaluateTableDriven(t *testing.T) {
	tests := []struct {
		name   string
		secret Code
		guess  Code
		blacks uint8
		whites uint8
	}{
		{"all same color match", code(3, 3, 3, 3), code(3, 3, 3, 3), 4, 0},
		{"all same color no match", code(1, 1, 1, 1), code(2, 2, 2, 2), 0, 0},
		{"boundary values low", code(1, 1, 1, 1), code(1, 1, 1, 1), 4, 0},
		{"boundary values high", code(6, 6, 6, 6), code(6, 6, 6, 6), 4, 0},
		{"one black rest no match", code(1, 2, 3, 4), code(1, 5, 5, 5), 1, 0},
		{"complex duplicate", code(1, 2, 1, 2), code(2, 1, 2, 1), 0, 4},
		{"partial duplicate", code(1, 1, 2, 3), code(1, 2, 1, 3), 2, 2},
		{"guess has extra duplicates", code(1, 2, 3, 4), code(1, 1, 1, 1), 1, 0},
		{"secret has extra duplicates", code(1, 1, 1, 1), code(1, 2, 3, 4), 1, 0},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			fb := Evaluate(tt.secret, tt.guess)
			if fb.Blacks != tt.blacks {
				t.Errorf("blacks: got %d, want %d", fb.Blacks, tt.blacks)
			}
			if fb.Whites != tt.whites {
				t.Errorf("whites: got %d, want %d", fb.Whites, tt.whites)
			}
		})
	}
}

// Test GameStatus JSON marshaling produces correct snake_case strings.
func TestGameStatusJSON(t *testing.T) {
	tests := []struct {
		status   GameStatus
		expected string
	}{
		{StatusInProgress, `"in_progress"`},
		{StatusWon, `"won"`},
		{StatusLost, `"lost"`},
	}

	for _, tt := range tests {
		t.Run(string(tt.status), func(t *testing.T) {
			data, err := json.Marshal(tt.status)
			if err != nil {
				t.Fatalf("marshal error: %v", err)
			}
			if string(data) != tt.expected {
				t.Errorf("got %s, want %s", string(data), tt.expected)
			}

			var status GameStatus
			if err := json.Unmarshal(data, &status); err != nil {
				t.Fatalf("unmarshal error: %v", err)
			}
			if status != tt.status {
				t.Errorf("round-trip: got %s, want %s", status, tt.status)
			}
		})
	}
}

// Test NewGame generates valid secret (all pegs in range).
func TestNewGameGeneratesValidSecret(t *testing.T) {
	for i := 0; i < 100; i++ {
		g := NewGame("test")
		for j, p := range g.Secret {
			if uint8(p) < 1 || uint8(p) > NumColors {
				t.Errorf("game %d, peg %d: value %d out of range [1, %d]", i, j, p, NumColors)
			}
		}
	}
}

func TestParseCodeValid(t *testing.T) {
	c, err := ParseCode([]uint8{1, 2, 3, 4})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	expected := code(1, 2, 3, 4)
	if c != expected {
		t.Errorf("got %v, want %v", c, expected)
	}
}

func TestParseCodeInvalidPeg(t *testing.T) {
	_, err := ParseCode([]uint8{1, 2, 3, 7})
	if err == nil {
		t.Error("expected error for peg value 7")
	}
	if !errors.Is(err, ErrInvalidPeg) {
		t.Errorf("expected ErrInvalidPeg, got %v", err)
	}
}

func TestCodeToSlice(t *testing.T) {
	c := code(1, 2, 3, 4)
	s := CodeToSlice(c)
	expected := []uint8{1, 2, 3, 4}
	for i, v := range expected {
		if s[i] != v {
			t.Errorf("s[%d] = %d, want %d", i, s[i], v)
		}
	}
}

func TestGameTurnsRecorded(t *testing.T) {
	g := NewGameWithSecret("test", code(1, 2, 3, 4))
	_, _ = g.Guess(code(5, 5, 5, 5))
	_, _ = g.Guess(code(6, 6, 6, 6))

	if len(g.Turns) != 2 {
		t.Fatalf("expected 2 turns, got %d", len(g.Turns))
	}
	if g.Turns[0].Attempt != 1 {
		t.Errorf("first turn attempt: got %d, want 1", g.Turns[0].Attempt)
	}
	if g.Turns[1].Attempt != 2 {
		t.Errorf("second turn attempt: got %d, want 2", g.Turns[1].Attempt)
	}
}

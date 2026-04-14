package analysis

import (
	"testing"

	"github.com/j4bberwocky/mastermind/mastermind-go/internal/game"
)

func code(a, b, c, d uint8) game.Code {
	return game.Code{game.Peg(a), game.Peg(b), game.Peg(c), game.Peg(d)}
}

func TestAllCodesCount(t *testing.T) {
	codes := AllCodes()
	if len(codes) != 1296 {
		t.Errorf("expected 1296 codes, got %d", len(codes))
	}
}

func TestFilterCandidates(t *testing.T) {
	all := AllCodes()
	guess := code(1, 1, 2, 2)
	fb := game.Feedback{Blacks: 2, Whites: 0}
	filtered := FilterCandidates(all, guess, fb)

	// Every remaining code should produce exactly that feedback against guess
	for _, c := range filtered {
		got := game.Evaluate(c, guess)
		if got != fb {
			t.Errorf("code %v produced feedback %v, expected %v", c, got, fb)
		}
	}

	if len(filtered) == 0 {
		t.Error("expected at least one filtered candidate")
	}
}

func TestAnalyzeWonGameOptimal(t *testing.T) {
	g := game.NewGameWithSecret("test", code(1, 2, 3, 4))

	// Simulate known-good guesses
	g.Guess(code(1, 1, 2, 2)) // well-known optimal first guess
	g.Guess(code(1, 2, 3, 4)) // win

	analysis := AnalyzeGame(g)
	if analysis.Status != game.StatusWon {
		t.Errorf("expected Won status, got %s", analysis.Status)
	}
	if analysis.TotalAttempts != 2 {
		t.Errorf("expected 2 attempts, got %d", analysis.TotalAttempts)
	}
	if analysis.OptimalityScore == 0 {
		t.Error("expected non-zero optimality score")
	}
}

func TestAnalyzeLostGame(t *testing.T) {
	g := game.NewGameWithSecret("test", code(1, 2, 3, 4))

	for i := 0; i < game.MaxAttempts; i++ {
		g.Guess(code(5, 5, 5, 5))
	}

	analysis := AnalyzeGame(g)
	if analysis.Status != game.StatusLost {
		t.Errorf("expected Lost status, got %s", analysis.Status)
	}
	expected := []uint8{1, 2, 3, 4}
	for i, v := range expected {
		if analysis.Secret[i] != v {
			t.Errorf("secret[%d] = %d, want %d", i, analysis.Secret[i], v)
		}
	}
}

func TestMinimaxScoreSingleCandidate(t *testing.T) {
	candidates := []game.Code{code(1, 2, 3, 4)}
	score := MinimaxScore(code(1, 2, 3, 4), candidates)
	if score != 1 {
		t.Errorf("expected score 1, got %d", score)
	}
}

func TestBestMinimaxGuessSingleCandidate(t *testing.T) {
	candidates := []game.Code{code(3, 4, 5, 6)}
	bestGuess, bestScore := BestMinimaxGuess(candidates)
	if bestGuess != code(3, 4, 5, 6) {
		t.Errorf("expected best guess to be the only candidate, got %v", bestGuess)
	}
	if bestScore != 1 {
		t.Errorf("expected best score 1, got %d", bestScore)
	}
}

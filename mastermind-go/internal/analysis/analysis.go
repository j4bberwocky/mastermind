// Package analysis implements minimax-based game analysis for Mastermind.
package analysis

import (
	"fmt"

	"github.com/j4bberwocky/mastermind/mastermind-go/internal/game"
)

// TurnAnalysis holds the analysis result for a single turn.
type TurnAnalysis struct {
	Attempt            int           `json:"attempt"`
	Guess              []uint8       `json:"guess"`
	Feedback           game.Feedback `json:"feedback"`
	CandidatesBefore   int           `json:"candidates_before"`
	CandidatesAfter    int           `json:"candidates_after"`
	SuggestedGuess     []uint8       `json:"suggested_guess"`
	SuggestedWorstCase *int          `json:"suggested_worst_case"`
	ActualWorstCase    int           `json:"actual_worst_case"`
	WasOptimal         bool          `json:"was_optimal"`
}

// GameAnalysis holds the overall game analysis.
type GameAnalysis struct {
	GameID          string          `json:"game_id"`
	Status          game.GameStatus `json:"status"`
	TotalAttempts   int             `json:"total_attempts"`
	MaxAttempts     int             `json:"max_attempts"`
	Secret          []uint8         `json:"secret"`
	Turns           []TurnAnalysis  `json:"turns"`
	OptimalityScore uint8           `json:"optimality_score"`
	Summary         string          `json:"summary"`
}

// AllCodes generates all possible codes in the classic 4-peg, 6-color game.
func AllCodes() []game.Code {
	codes := make([]game.Code, 0, 1296) // 6^4
	for a := uint8(1); a <= game.NumColors; a++ {
		for b := uint8(1); b <= game.NumColors; b++ {
			for c := uint8(1); c <= game.NumColors; c++ {
				for d := uint8(1); d <= game.NumColors; d++ {
					codes = append(codes, game.Code{game.Peg(a), game.Peg(b), game.Peg(c), game.Peg(d)})
				}
			}
		}
	}
	return codes
}

// feedbackKey creates a key for a feedback pair to use in maps.
type feedbackKey struct {
	blacks, whites uint8
}

// MinimaxScore computes the worst-case partition size for a given guess
// against a set of remaining candidates.
func MinimaxScore(guess game.Code, candidates []game.Code) int {
	partition := make(map[feedbackKey]int)
	for _, secret := range candidates {
		fb := game.Evaluate(secret, guess)
		partition[feedbackKey{fb.Blacks, fb.Whites}]++
	}
	maxVal := 0
	for _, count := range partition {
		if count > maxVal {
			maxVal = count
		}
	}
	return maxVal
}

// BestMinimaxGuess finds the best guess from all 1296 codes using the minimax strategy,
// preferring candidates over non-candidates on ties.
func BestMinimaxGuess(candidates []game.Code) (game.Code, int) {
	if len(candidates) == 1 {
		return candidates[0], 1
	}

	all := AllCodes()
	candidateSet := make(map[game.Code]bool, len(candidates))
	for _, c := range candidates {
		candidateSet[c] = true
	}

	bestGuess := all[0]
	bestScore := int(^uint(0) >> 1) // max int
	bestIsCandidate := false

	for _, g := range all {
		score := MinimaxScore(g, candidates)
		isCandidate := candidateSet[g]

		better := score < bestScore ||
			(score == bestScore && isCandidate && !bestIsCandidate)

		if better {
			bestScore = score
			bestGuess = g
			bestIsCandidate = isCandidate
		}
	}

	return bestGuess, bestScore
}

// FilterCandidates filters a list of candidate codes to those consistent with the
// feedback received for a particular guess.
func FilterCandidates(candidates []game.Code, guess game.Code, feedback game.Feedback) []game.Code {
	var result []game.Code
	for _, candidate := range candidates {
		if game.Evaluate(candidate, guess) == feedback {
			result = append(result, candidate)
		}
	}
	return result
}

// AnalyzeGame analyzes a completed (won or lost) game.
func AnalyzeGame(g *game.Game) *GameAnalysis {
	all := AllCodes()
	candidates := make([]game.Code, len(all))
	copy(candidates, all)

	turnAnalyses := make([]TurnAnalysis, 0, len(g.Turns))
	suboptimalTurns := 0

	for _, turn := range g.Turns {
		candidatesBefore := len(candidates)
		guess, _ := game.ParseCode(turn.Guess)
		feedback := turn.Feedback

		actualWorstCase := MinimaxScore(guess, candidates)
		bestGuess, bestScore := BestMinimaxGuess(candidates)

		wasOptimal := actualWorstCase <= bestScore

		var suggestedGuess []uint8
		var suggestedWorstCase *int
		if !wasOptimal {
			suboptimalTurns++
			suggestedGuess = game.CodeToSlice(bestGuess)
			suggestedWorstCase = &bestScore
		}

		candidates = FilterCandidates(candidates, guess, feedback)
		candidatesAfter := len(candidates)

		turnAnalyses = append(turnAnalyses, TurnAnalysis{
			Attempt:            turn.Attempt,
			Guess:              turn.Guess,
			Feedback:           feedback,
			CandidatesBefore:   candidatesBefore,
			CandidatesAfter:    candidatesAfter,
			SuggestedGuess:     suggestedGuess,
			SuggestedWorstCase: suggestedWorstCase,
			ActualWorstCase:    actualWorstCase,
			WasOptimal:         wasOptimal,
		})
	}

	totalAttempts := len(g.Turns)
	var optimalityScore uint8
	if totalAttempts == 0 {
		optimalityScore = 100
	} else {
		optimalTurns := totalAttempts - suboptimalTurns
		optimalityScore = uint8((optimalTurns * 100) / totalAttempts)
	}

	summary := buildSummary(g, turnAnalyses, optimalityScore)

	return &GameAnalysis{
		GameID:          g.ID,
		Status:          g.Status,
		TotalAttempts:   totalAttempts,
		MaxAttempts:     game.MaxAttempts,
		Secret:          game.CodeToSlice(g.Secret),
		Turns:           turnAnalyses,
		OptimalityScore: optimalityScore,
		Summary:         summary,
	}
}

func buildSummary(g *game.Game, turns []TurnAnalysis, score uint8) string {
	switch g.Status {
	case game.StatusInProgress:
		return "Game is still in progress."
	case game.StatusWon:
		suboptimal := 0
		for _, t := range turns {
			if !t.WasOptimal {
				suboptimal++
			}
		}
		if suboptimal == 0 {
			return fmt.Sprintf(
				"Excellent! You solved the puzzle in %d attempt(s) with optimal play (score: %d/100).",
				len(g.Turns), score,
			)
		}
		return fmt.Sprintf(
			"You solved the puzzle in %d attempt(s) with an optimality score of %d/100. "+
				"%d turn(s) had a suboptimal guess — check the per-turn suggestions.",
			len(g.Turns), score, suboptimal,
		)
	case game.StatusLost:
		secret := game.CodeToSlice(g.Secret)
		return fmt.Sprintf(
			"You ran out of guesses after %d attempt(s) (score: %d/100). "+
				"The secret was %v. Review the suggestions to improve your strategy.",
			len(g.Turns), score, secret,
		)
	default:
		return "Unknown game status."
	}
}

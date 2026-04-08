use serde::{Deserialize, Serialize};

use crate::game::{evaluate, parse_code, Code, Feedback, Game, GameStatus, NUM_COLORS};

/// All possible codes in the classic 4-peg, 6-color game.
fn all_codes() -> Vec<Code> {
    let mut codes = Vec::with_capacity(1296); // 6^4
    for a in 1..=NUM_COLORS {
        for b in 1..=NUM_COLORS {
            for c in 1..=NUM_COLORS {
                for d in 1..=NUM_COLORS {
                    codes.push(parse_code(&[a, b, c, d]).unwrap());
                }
            }
        }
    }
    codes
}

/// Analysis result for a single turn
#[derive(Debug, Serialize, Deserialize)]
pub struct TurnAnalysis {
    pub attempt: usize,
    pub guess: Vec<u8>,
    pub feedback: Feedback,
    /// Number of remaining possible secrets before this guess
    pub candidates_before: usize,
    /// Number of remaining possible secrets after this guess
    pub candidates_after: usize,
    /// A suggested optimal guess (minimax) if the player's guess was suboptimal
    pub suggested_guess: Option<Vec<u8>>,
    /// The minimax worst-case remaining candidates for the suggested guess
    pub suggested_worst_case: Option<usize>,
    /// The worst-case remaining candidates for the player's actual guess
    pub actual_worst_case: usize,
    /// True when the player's guess was an optimal move
    pub was_optimal: bool,
}

/// Overall game analysis
#[derive(Debug, Serialize, Deserialize)]
pub struct GameAnalysis {
    pub game_id: String,
    pub status: GameStatus,
    pub total_attempts: usize,
    pub max_attempts: usize,
    pub secret: Vec<u8>,
    pub turns: Vec<TurnAnalysis>,
    /// A rating 0–100 (100 = perfect play)
    pub optimality_score: u8,
    /// Human-readable summary
    pub summary: String,
}

/// Computes the minimax score (worst-case partition size) for a given guess
/// against a set of remaining candidates.
fn minimax_score(guess: Code, candidates: &[Code]) -> usize {
    let mut partition: std::collections::HashMap<(u8, u8), usize> =
        std::collections::HashMap::new();
    for &secret in candidates {
        let fb = evaluate(secret, guess);
        *partition.entry((fb.blacks, fb.whites)).or_insert(0) += 1;
    }
    partition.values().copied().max().unwrap_or(0)
}

/// Finds the best guess from all 1296 codes using the minimax strategy,
/// preferring candidates over non-candidates on ties.
fn best_minimax_guess(candidates: &[Code]) -> (Code, usize) {
    if candidates.len() == 1 {
        return (candidates[0], 1);
    }

    let all = all_codes();
    let candidate_set: std::collections::HashSet<_> = candidates.iter().copied().collect();

    let mut best_guess = all[0];
    let mut best_score = usize::MAX;
    let mut best_is_candidate = false;

    for &g in &all {
        let score = minimax_score(g, candidates);
        let is_candidate = candidate_set.contains(&g);

        let better = score < best_score
            || (score == best_score && is_candidate && !best_is_candidate);

        if better {
            best_score = score;
            best_guess = g;
            best_is_candidate = is_candidate;
        }
    }

    (best_guess, best_score)
}

/// Filters a list of candidate codes to those consistent with the feedback
/// received for a particular guess.
fn filter_candidates(candidates: &[Code], guess: Code, feedback: Feedback) -> Vec<Code> {
    candidates
        .iter()
        .copied()
        .filter(|&candidate| evaluate(candidate, guess) == feedback)
        .collect()
}

/// Analyzes a completed (won or lost) game.
pub fn analyze_game(game: &Game) -> GameAnalysis {
    let all = all_codes();
    let mut candidates: Vec<Code> = all.clone();

    let mut turn_analyses: Vec<TurnAnalysis> = Vec::new();
    let mut suboptimal_turns: usize = 0;

    for turn in &game.turns {
        let candidates_before = candidates.len();
        let guess = parse_code(&turn.guess).expect("stored guess is always valid");
        let feedback = turn.feedback;

        let actual_worst_case = minimax_score(guess, &candidates);
        let (best_guess, best_score) = best_minimax_guess(&candidates);

        let was_optimal = actual_worst_case <= best_score;

        let suggested_guess = if was_optimal {
            None
        } else {
            suboptimal_turns += 1;
            Some(best_guess.map(|p| p.0).to_vec())
        };

        candidates = filter_candidates(&candidates, guess, feedback);
        let candidates_after = candidates.len();

        turn_analyses.push(TurnAnalysis {
            attempt: turn.attempt,
            guess: turn.guess.clone(),
            feedback,
            candidates_before,
            candidates_after,
            suggested_guess,
            suggested_worst_case: if was_optimal { None } else { Some(best_score) },
            actual_worst_case,
            was_optimal,
        });
    }

    let total_attempts = game.turns.len();
    let optimality_score = if total_attempts == 0 {
        100
    } else {
        let optimal_turns = total_attempts - suboptimal_turns;
        ((optimal_turns * 100) / total_attempts) as u8
    };

    let summary = build_summary(game, &turn_analyses, optimality_score);

    GameAnalysis {
        game_id: game.id.clone(),
        status: game.status.clone(),
        total_attempts,
        max_attempts: crate::game::MAX_ATTEMPTS,
        secret: game.secret.map(|p| p.0).to_vec(),
        turns: turn_analyses,
        optimality_score,
        summary,
    }
}

fn build_summary(game: &Game, turns: &[TurnAnalysis], score: u8) -> String {
    match &game.status {
        GameStatus::InProgress => "Game is still in progress.".to_string(),
        GameStatus::Won => {
            let suboptimal: Vec<_> = turns.iter().filter(|t| !t.was_optimal).collect();
            if suboptimal.is_empty() {
                format!(
                    "Excellent! You solved the puzzle in {} attempt(s) with optimal play (score: {}/100).",
                    game.turns.len(),
                    score
                )
            } else {
                format!(
                    "You solved the puzzle in {} attempt(s) with an optimality score of {}/100. \
                     {} turn(s) had a suboptimal guess — check the per-turn suggestions.",
                    game.turns.len(),
                    score,
                    suboptimal.len()
                )
            }
        }
        GameStatus::Lost => {
            format!(
                "You ran out of guesses after {} attempt(s) (score: {}/100). \
                 The secret was {:?}. Review the suggestions to improve your strategy.",
                game.turns.len(),
                score,
                game.secret.map(|p| p.0)
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Game, Peg};

    fn code(a: u8, b: u8, c: u8, d: u8) -> Code {
        [Peg(a), Peg(b), Peg(c), Peg(d)]
    }

    #[test]
    fn test_all_codes_count() {
        assert_eq!(all_codes().len(), 1296);
    }

    #[test]
    fn test_filter_candidates() {
        let all = all_codes();
        let guess = code(1, 1, 2, 2);
        // If we get 2 blacks, 0 whites -> many consistent codes
        let fb = Feedback { blacks: 2, whites: 0 };
        let filtered = filter_candidates(&all, guess, fb);
        // Every remaining code should produce exactly that feedback against guess
        for c in &filtered {
            assert_eq!(evaluate(*c, guess), fb);
        }
    }

    #[test]
    fn test_analyze_won_game_optimal() {
        // Use Knuth's known optimal first guess 1122
        let mut game = Game::new("test".into());
        game.secret = code(1, 2, 3, 4);

        // Simulate known-good guesses
        game.guess(code(1, 1, 2, 2)).unwrap(); // well-known optimal first guess
        game.guess(code(1, 2, 3, 4)).unwrap(); // win

        let analysis = analyze_game(&game);
        assert_eq!(analysis.status, GameStatus::Won);
        assert_eq!(analysis.total_attempts, 2);
        assert!(analysis.optimality_score > 0);
    }

    #[test]
    fn test_analyze_lost_game() {
        let mut game = Game::new("test".into());
        game.secret = code(1, 2, 3, 4);

        for _ in 0..crate::game::MAX_ATTEMPTS {
            let _ = game.guess(code(5, 5, 5, 5));
        }

        let analysis = analyze_game(&game);
        assert_eq!(analysis.status, GameStatus::Lost);
        assert_eq!(analysis.secret, vec![1, 2, 3, 4]);
    }
}

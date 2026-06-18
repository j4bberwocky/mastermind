//! Information-theoretic feasibility check for Mastermind puzzles.
//!
//! A puzzle with `R` remaining attempts after the initial guesses is accepted
//! iff `R >= ceil(log_P(N))`, where `P = (L+1)(L+2)/2 - 1` is the count of valid
//! feedback patterns for code length `L` and `N` is the number of codes in the
//! configured domain that are consistent with all initial feedback. `N` is
//! computed by exhaustive enumeration (≤ 16.7M for 8×8 allow_duplicates).

use crate::{evaluate_guess, Feedback, Settings};

/// Count of valid `(blacks, whites)` feedback pairs for code length `L`.
/// All pairs satisfying `b + w <= L` minus the impossible `(L-1, 1)`.
pub fn pattern_count(code_length: usize) -> usize {
    (code_length + 1) * (code_length + 2) / 2 - 1
}

/// Minimum guesses needed, in the worst case, to distinguish `candidates`
/// codes. Information-theoretic lower bound `ceil(log_P(N))`.
pub fn min_attempts_needed(candidates: usize, code_length: usize) -> usize {
    if candidates <= 1 {
        return 0;
    }
    let p = pattern_count(code_length) as f64;
    ((candidates as f64).log(p)).ceil() as usize
}

/// Enumerate the configured code domain and count how many codes are
/// consistent with all initial feedback.
pub fn count_candidates(
    settings: &Settings,
    initial_guesses: &[Vec<u8>],
    initial_feedback: &[Feedback],
) -> usize {
    debug_assert_eq!(initial_guesses.len(), initial_feedback.len());
    let l = settings.code_length;
    let c = settings.num_colors;
    let mut count = 0usize;

    let is_consistent = |code: &[u8]| -> bool {
        for (g, expected) in initial_guesses.iter().zip(initial_feedback.iter()) {
            let actual = evaluate_guess(g, code);
            if actual.blacks != expected.blacks || actual.whites != expected.whites {
                return false;
            }
        }
        true
    };

    if settings.allow_duplicates {
        let total = (c as u64).pow(l as u32) as usize;
        let mut code = vec![0u8; l];
        for _ in 0..total {
            if is_consistent(&code) {
                count += 1;
            }
            // base-`c` increment (least significant at index 0)
            let mut i = 0;
            while i < l {
                code[i] = code[i].wrapping_add(1);
                if (code[i] as usize) < c {
                    break;
                }
                code[i] = 0;
                i += 1;
            }
        }
    } else {
        let mut code = vec![0u8; l];
        let mut used = vec![false; c];
        permute(&mut code, &mut used, 0, l, c as u8, &mut |c| {
            if is_consistent(c) {
                count += 1;
            }
        });
    }

    count
}

fn permute(
    code: &mut [u8],
    used: &mut [bool],
    pos: usize,
    l: usize,
    c: u8,
    f: &mut impl FnMut(&[u8]),
) {
    if pos == l {
        f(code);
        return;
    }
    for v in 0..c {
        let idx = v as usize;
        if used[idx] {
            continue;
        }
        used[idx] = true;
        code[pos] = v;
        permute(code, used, pos + 1, l, c, f);
        used[idx] = false;
    }
}

/// Returns `Err` with a user-facing message if the puzzle is information-
/// theoretically infeasible in the worst case.
pub fn validate_solvable(
    settings: &Settings,
    initial_guesses: &[Vec<u8>],
    initial_feedback: &[Feedback],
) -> Result<(), String> {
    let n = count_candidates(settings, initial_guesses, initial_feedback);
    let remaining = settings.max_attempts.saturating_sub(initial_guesses.len());
    let min_needed = min_attempts_needed(n, settings.code_length);
    if remaining < min_needed {
        return Err(format!(
            "Puzzle non risolvibile nel caso peggiore: servono almeno {min_needed} tentativi, ne hai {remaining}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(l: usize, c: usize, dup: bool, max: usize) -> Settings {
        Settings {
            code_length: l,
            num_colors: c,
            allow_duplicates: dup,
            max_attempts: max,
        }
    }

    #[test]
    fn pattern_count_matches_table() {
        assert_eq!(pattern_count(2), 5);
        assert_eq!(pattern_count(3), 9);
        assert_eq!(pattern_count(4), 14);
        assert_eq!(pattern_count(5), 20);
        assert_eq!(pattern_count(6), 27);
        assert_eq!(pattern_count(7), 35);
        assert_eq!(pattern_count(8), 44);
    }

    #[test]
    fn min_attempts_4x6_classic_is_3() {
        // log_14(1296) ≈ 2.72 → ceil = 3
        assert_eq!(min_attempts_needed(1296, 4), 3);
    }

    #[test]
    fn min_attempts_single_candidate_is_zero() {
        assert_eq!(min_attempts_needed(1, 4), 0);
        assert_eq!(min_attempts_needed(0, 4), 0);
    }

    #[test]
    fn count_no_initial_guesses_equals_full_domain_with_dup() {
        let n = count_candidates(&s(4, 6, true, 10), &[], &[]);
        assert_eq!(n, 1296);
    }

    #[test]
    fn count_no_initial_guesses_equals_permutations_no_dup() {
        // P(6, 4) = 6 * 5 * 4 * 3 = 360
        let n = count_candidates(&s(4, 6, false, 10), &[], &[]);
        assert_eq!(n, 360);
    }

    #[test]
    fn count_with_perfect_match_yields_one() {
        // If an initial guess equals the secret code, feedback is (L, 0) and
        // only the code itself can satisfy that — count must be 1.
        let code = vec![0u8, 1, 2, 3];
        let fb = vec![Feedback { blacks: 4, whites: 0 }];
        let n = count_candidates(&s(4, 6, true, 10), &[code.clone()], &fb);
        assert_eq!(n, 1);
    }

    #[test]
    fn count_with_zero_zero_feedback_excludes_used_colors() {
        // Guess [0,1,2,3] with feedback (0, 0): the code uses none of 0,1,2,3
        // at all positions, AND no color overlap whatsoever. For 4-peg / 6-color
        // / dup, the only allowed colors are {4, 5} → 2^4 = 16 candidates.
        let guess = vec![0u8, 1, 2, 3];
        let fb = vec![Feedback { blacks: 0, whites: 0 }];
        let n = count_candidates(&s(4, 6, true, 10), &[guess], &fb);
        assert_eq!(n, 16);
    }

    #[test]
    fn validate_classic_4x6_passes() {
        assert!(validate_solvable(&s(4, 6, true, 10), &[], &[]).is_ok());
    }

    #[test]
    fn validate_rejects_infeasible_2_attempts() {
        let err = validate_solvable(&s(4, 6, true, 2), &[], &[]).unwrap_err();
        assert!(
            err.contains("almeno 3"),
            "expected message to mention min=3, got: {err}"
        );
    }

    #[test]
    fn validate_rejects_8x8_with_4_attempts() {
        // 8^8 ≈ 16.7M, P=44, log_44 ≈ 4.4 → min 5. 4 attempts is short.
        let err = validate_solvable(&s(8, 8, true, 4), &[], &[]).unwrap_err();
        assert!(
            err.contains("almeno 5"),
            "expected message to mention min=5, got: {err}"
        );
    }

    #[test]
    fn validate_passes_8x8_with_5_attempts() {
        assert!(validate_solvable(&s(8, 8, true, 5), &[], &[]).is_ok());
    }
}

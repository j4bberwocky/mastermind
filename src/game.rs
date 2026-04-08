use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fmt;

pub const CODE_LENGTH: usize = 4;
pub const NUM_COLORS: u8 = 6;
pub const MAX_ATTEMPTS: usize = 10;

/// Represents a single color peg (1–6)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Peg(pub u8);

impl Peg {
    /// Creates a new Peg, returning an error if the value is out of range.
    pub fn new(value: u8) -> Result<Self, GameError> {
        if (1..=NUM_COLORS).contains(&value) {
            Ok(Self(value))
        } else {
            Err(GameError::InvalidPeg(value))
        }
    }
}

impl fmt::Display for Peg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A combination of CODE_LENGTH pegs
pub type Code = [Peg; CODE_LENGTH];

/// Result of comparing a guess against the secret code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Feedback {
    /// Pegs with correct color AND correct position
    pub blacks: u8,
    /// Pegs with correct color but wrong position
    pub whites: u8,
}

impl Feedback {
    /// Returns true when the guess perfectly matched the secret
    pub fn is_perfect(&self) -> bool {
        self.blacks == CODE_LENGTH as u8
    }
}

/// A single recorded turn: the guess and its feedback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    pub attempt: usize,
    pub guess: Vec<u8>,
    pub feedback: Feedback,
}

/// Current status of a game
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameStatus {
    InProgress,
    Won,
    Lost,
}

/// Full game state (stored in memory per session)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub id: String,
    pub secret: Code,
    pub turns: Vec<Turn>,
    pub status: GameStatus,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Game {
    /// Creates a new game with a randomly generated secret code.
    pub fn new(id: String) -> Self {
        let secret = Self::generate_secret();
        Self {
            id,
            secret,
            turns: Vec::new(),
            status: GameStatus::InProgress,
            started_at: chrono::Utc::now(),
            finished_at: None,
        }
    }

    /// Generates a random secret code.
    fn generate_secret() -> Code {
        let mut rng = rand::thread_rng();
        // Using array::from_fn would require const generics – this is cleaner
        let values: [u8; CODE_LENGTH] =
            std::array::from_fn(|_| rng.gen_range(1..=NUM_COLORS));
        values.map(Peg)
    }

    /// Makes a guess, records it and returns the feedback.
    pub fn guess(&mut self, code: Code) -> Result<Feedback, GameError> {
        if self.status != GameStatus::InProgress {
            return Err(GameError::GameOver);
        }

        let feedback = evaluate(self.secret, code);
        let attempt = self.turns.len() + 1;

        self.turns.push(Turn {
            attempt,
            guess: code.map(|p| p.0).to_vec(),
            feedback,
        });

        if feedback.is_perfect() {
            self.status = GameStatus::Won;
            self.finished_at = Some(chrono::Utc::now());
        } else if self.turns.len() >= MAX_ATTEMPTS {
            self.status = GameStatus::Lost;
            self.finished_at = Some(chrono::Utc::now());
        }

        Ok(feedback)
    }

    /// Returns the secret only if the game is over.
    pub fn revealed_secret(&self) -> Option<Vec<u8>> {
        if self.status != GameStatus::InProgress {
            Some(self.secret.map(|p| p.0).to_vec())
        } else {
            None
        }
    }
}

/// Evaluates a guess against the secret and returns a Feedback struct.
pub fn evaluate(secret: Code, guess: Code) -> Feedback {
    let mut blacks: u8 = 0;
    let mut secret_remaining = [0u8; (NUM_COLORS + 1) as usize];
    let mut guess_remaining = [0u8; (NUM_COLORS + 1) as usize];

    for i in 0..CODE_LENGTH {
        if secret[i] == guess[i] {
            blacks += 1;
        } else {
            secret_remaining[secret[i].0 as usize] += 1;
            guess_remaining[guess[i].0 as usize] += 1;
        }
    }

    let whites: u8 = (1..=(NUM_COLORS as usize))
        .map(|c| secret_remaining[c].min(guess_remaining[c]))
        .sum();

    Feedback { blacks, whites }
}

/// Parses a slice of u8 values into a Code, validating each peg.
pub fn parse_code(values: &[u8]) -> Result<Code, GameError> {
    if values.len() != CODE_LENGTH {
        return Err(GameError::InvalidCodeLength(values.len()));
    }
    let pegs: Result<Vec<Peg>, _> = values.iter().map(|&v| Peg::new(v)).collect();
    let pegs = pegs?;
    Ok([pegs[0], pegs[1], pegs[2], pegs[3]])
}

/// Domain errors for the game logic
#[derive(Debug, thiserror::Error)]
pub enum GameError {
    #[error("Game is already over")]
    GameOver,
    #[error("Invalid peg value: {0} (must be 1–{NUM_COLORS})")]
    InvalidPeg(u8),
    #[error("Code must have exactly {CODE_LENGTH} pegs, got {0}")]
    InvalidCodeLength(usize),
    #[error("Game not found")]
    NotFound,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peg(v: u8) -> Peg {
        Peg::new(v).unwrap()
    }

    fn code(a: u8, b: u8, c: u8, d: u8) -> Code {
        [peg(a), peg(b), peg(c), peg(d)]
    }

    #[test]
    fn test_perfect_match() {
        let secret = code(1, 2, 3, 4);
        let guess = code(1, 2, 3, 4);
        let fb = evaluate(secret, guess);
        assert_eq!(fb.blacks, 4);
        assert_eq!(fb.whites, 0);
        assert!(fb.is_perfect());
    }

    #[test]
    fn test_no_match() {
        let secret = code(1, 1, 1, 1);
        let guess = code(2, 2, 2, 2);
        let fb = evaluate(secret, guess);
        assert_eq!(fb.blacks, 0);
        assert_eq!(fb.whites, 0);
    }

    #[test]
    fn test_all_whites() {
        let secret = code(1, 2, 3, 4);
        let guess = code(4, 3, 2, 1);
        let fb = evaluate(secret, guess);
        assert_eq!(fb.blacks, 0);
        assert_eq!(fb.whites, 4);
    }

    #[test]
    fn test_mixed() {
        let secret = code(1, 2, 3, 4);
        let guess = code(1, 3, 5, 6);
        let fb = evaluate(secret, guess);
        assert_eq!(fb.blacks, 1); // position 0
        assert_eq!(fb.whites, 1); // 3 is in secret but wrong position
    }

    #[test]
    fn test_duplicate_handling() {
        // Classic edge-case: secret [1,1,2,2], guess [1,1,1,1]
        let secret = code(1, 1, 2, 2);
        let guess = code(1, 1, 1, 1);
        let fb = evaluate(secret, guess);
        assert_eq!(fb.blacks, 2);
        assert_eq!(fb.whites, 0); // only 2 ones in secret, both already matched as black
    }

    #[test]
    fn test_game_won() {
        let mut game = Game::new("test".into());
        game.secret = code(1, 2, 3, 4);
        let fb = game.guess(code(1, 2, 3, 4)).unwrap();
        assert!(fb.is_perfect());
        assert_eq!(game.status, GameStatus::Won);
        assert!(game.finished_at.is_some());
    }

    #[test]
    fn test_game_lost_after_max_attempts() {
        let mut game = Game::new("test".into());
        game.secret = code(1, 2, 3, 4);
        for _ in 0..MAX_ATTEMPTS {
            let _ = game.guess(code(5, 5, 5, 5));
        }
        assert_eq!(game.status, GameStatus::Lost);
    }

    #[test]
    fn test_cannot_guess_after_game_over() {
        let mut game = Game::new("test".into());
        game.secret = code(1, 2, 3, 4);
        let _ = game.guess(code(1, 2, 3, 4));
        let result = game.guess(code(1, 2, 3, 4));
        assert!(matches!(result, Err(GameError::GameOver)));
    }

    #[test]
    fn test_invalid_peg() {
        assert!(Peg::new(0).is_err());
        assert!(Peg::new(7).is_err());
        assert!(Peg::new(1).is_ok());
        assert!(Peg::new(6).is_ok());
    }

    #[test]
    fn test_parse_code_wrong_length() {
        assert!(parse_code(&[1, 2, 3]).is_err());
        assert!(parse_code(&[1, 2, 3, 4, 5]).is_err());
    }

    #[test]
    fn test_secret_revealed_only_when_done() {
        let mut game = Game::new("test".into());
        game.secret = code(1, 2, 3, 4);
        assert!(game.revealed_secret().is_none());
        let _ = game.guess(code(1, 2, 3, 4));
        assert_eq!(game.revealed_secret(), Some(vec![1, 2, 3, 4]));
    }
}

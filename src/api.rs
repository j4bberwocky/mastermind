use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    analysis::analyze_game,
    game::{parse_code, Game, GameStatus, CODE_LENGTH, NUM_COLORS},
    state::AppState,
};

// ── Response types ───────────────────────────────────────────────────────────

/// Returned when a new game is created.
#[derive(Debug, Serialize)]
pub struct NewGameResponse {
    pub game_id: String,
    pub code_length: usize,
    pub num_colors: u8,
    pub max_attempts: usize,
    pub message: String,
}

/// Public view of the game state (secret is hidden while in progress).
#[derive(Debug, Serialize)]
pub struct GameStateResponse {
    pub game_id: String,
    pub status: GameStatus,
    pub attempts_used: usize,
    pub max_attempts: usize,
    pub turns: Vec<TurnResponse>,
    /// Revealed only when the game is over.
    pub secret: Option<Vec<u8>>,
}

#[derive(Debug, Serialize)]
pub struct TurnResponse {
    pub attempt: usize,
    pub guess: Vec<u8>,
    pub blacks: u8,
    pub whites: u8,
}

/// Returned after each guess.
#[derive(Debug, Serialize)]
pub struct GuessResponse {
    pub attempt: usize,
    pub blacks: u8,
    pub whites: u8,
    pub status: GameStatus,
    /// Only set when the game ends.
    pub secret: Option<Vec<u8>>,
    pub message: String,
}

// ── Request types ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct GuessRequest {
    /// A list of CODE_LENGTH integers, each in [1, NUM_COLORS].
    pub guess: Vec<u8>,
}

// ── Error helper ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

fn err(status: StatusCode, message: impl Into<String>) -> impl IntoResponse {
    (status, Json(ErrorResponse { error: message.into() }))
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// POST /api/game  — Start a new game
pub async fn create_game(State(state): State<AppState>) -> impl IntoResponse {
    let id = Uuid::new_v4().to_string();
    let game = Game::new(id.clone());
    state.games.insert(id.clone(), game);

    tracing::info!("New game created: {}", id);

    (
        StatusCode::CREATED,
        Json(NewGameResponse {
            game_id: id,
            code_length: CODE_LENGTH,
            num_colors: NUM_COLORS,
            max_attempts: crate::game::MAX_ATTEMPTS,
            message: format!(
                "New game started! Guess the {CODE_LENGTH}-peg code using colors 1–{NUM_COLORS}. \
                 You have {} attempts.",
                crate::game::MAX_ATTEMPTS
            ),
        }),
    )
}

/// GET /api/game/:id  — Get current game state
pub async fn get_game(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Some(game) = state.games.get(&id) else {
        return err(StatusCode::NOT_FOUND, "Game not found").into_response();
    };

    let turns = game
        .turns
        .iter()
        .map(|t| TurnResponse {
            attempt: t.attempt,
            guess: t.guess.clone(),
            blacks: t.feedback.blacks,
            whites: t.feedback.whites,
        })
        .collect();

    Json(GameStateResponse {
        game_id: game.id.clone(),
        status: game.status.clone(),
        attempts_used: game.turns.len(),
        max_attempts: crate::game::MAX_ATTEMPTS,
        turns,
        secret: game.revealed_secret(),
    })
    .into_response()
}

/// POST /api/game/:id/guess  — Submit a guess
pub async fn make_guess(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<GuessRequest>,
) -> impl IntoResponse {
    let Some(mut game_ref) = state.games.get_mut(&id) else {
        return err(StatusCode::NOT_FOUND, "Game not found").into_response();
    };

    let code = match parse_code(&body.guess) {
        Ok(c) => c,
        Err(e) => return err(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };

    match game_ref.guess(code) {
        Err(e) => err(StatusCode::CONFLICT, e.to_string()).into_response(),
        Ok(feedback) => {
            let status = game_ref.status.clone();
            let secret = game_ref.revealed_secret();
            let attempt = game_ref.turns.len();

            let message = match &status {
                GameStatus::Won => format!(
                    "🎉 Congratulations! You guessed the secret in {} attempt(s)!",
                    attempt
                ),
                GameStatus::Lost => format!(
                    "Game over! The secret was {:?}. Better luck next time!",
                    secret.as_deref().unwrap_or(&[])
                ),
                GameStatus::InProgress => format!(
                    "{} black(s), {} white(s). {} attempt(s) remaining.",
                    feedback.blacks,
                    feedback.whites,
                    crate::game::MAX_ATTEMPTS - attempt
                ),
            };

            tracing::info!(
                game_id = %id,
                attempt = attempt,
                blacks = feedback.blacks,
                whites = feedback.whites,
                status = ?status,
                "Guess evaluated"
            );

            Json(GuessResponse {
                attempt,
                blacks: feedback.blacks,
                whites: feedback.whites,
                status,
                secret,
                message,
            })
            .into_response()
        }
    }
}

/// POST /api/game/:id/analyze  — Analyze a finished game
pub async fn analyze(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Some(game) = state.games.get(&id) else {
        return err(StatusCode::NOT_FOUND, "Game not found").into_response();
    };

    if game.status == GameStatus::InProgress {
        return err(
            StatusCode::CONFLICT,
            "Game is still in progress — finish the game before requesting analysis.",
        )
        .into_response();
    }

    let analysis = analyze_game(&game);

    tracing::info!(game_id = %id, score = analysis.optimality_score, "Game analyzed");

    Json(analysis).into_response()
}

/// GET /api/game/:id/export  — Export game log as JSON (portable format)
pub async fn export_game(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Some(game) = state.games.get(&id) else {
        return err(StatusCode::NOT_FOUND, "Game not found").into_response();
    };

    // Serialise the full game record (secret hidden while in progress)
    let export = GameExport::from(&*game);
    Json(export).into_response()
}

/// Portable game export format (similar to the Mastermind game log convention)
#[derive(Debug, Serialize)]
pub struct GameExport {
    pub schema_version: &'static str,
    pub game_id: String,
    pub variant: GameVariant,
    pub status: GameStatus,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
    pub turns: Vec<ExportTurn>,
    /// Revealed only when the game is over
    pub secret: Option<Vec<u8>>,
}

#[derive(Debug, Serialize)]
pub struct GameVariant {
    pub code_length: usize,
    pub num_colors: u8,
    pub max_attempts: usize,
    pub allow_repetition: bool,
}

#[derive(Debug, Serialize)]
pub struct ExportTurn {
    pub attempt: usize,
    pub guess: Vec<u8>,
    pub blacks: u8,
    pub whites: u8,
}

impl From<&Game> for GameExport {
    fn from(game: &Game) -> Self {
        Self {
            schema_version: "1.0",
            game_id: game.id.clone(),
            variant: GameVariant {
                code_length: CODE_LENGTH,
                num_colors: NUM_COLORS,
                max_attempts: crate::game::MAX_ATTEMPTS,
                allow_repetition: true,
            },
            status: game.status.clone(),
            started_at: game.started_at,
            finished_at: game.finished_at,
            turns: game
                .turns
                .iter()
                .map(|t| ExportTurn {
                    attempt: t.attempt,
                    guess: t.guess.clone(),
                    blacks: t.feedback.blacks,
                    whites: t.feedback.whites,
                })
                .collect(),
            secret: game.revealed_secret(),
        }
    }
}

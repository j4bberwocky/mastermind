//! Mastermind backend — Axum + Tokio + in-memory store.
//!
//! API surface
//!
//!   POST   /api/problems            create a problem
//!   GET    /api/problems            list problems (no codes)
//!   GET    /api/problems/:id        fetch one problem (no code)
//!   POST   /api/problems/:id/check  evaluate a guess against the secret code
//!   GET    /api/problems/:id/code   reveal the secret code (always permitted —
//!                                   the frontend only calls this once a game is
//!                                   over; tighten with a session token in prod)
//!
//! The frontend (the static SPA at the project root) is also served from this
//! same binary under `/`, so a single `cargo run` is enough to play in a
//! browser without CORS gymnastics.

use axum::{
    extract::{Path, State},
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};

// ──────────────────────────────────────────────────────────────
// Types
// ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    code_length: usize,
    num_colors: usize,
    allow_duplicates: bool,
    max_attempts: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct Feedback {
    blacks: usize,
    whites: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Problem {
    id: String,
    /// Secret — never serialised in the public view.
    #[serde(skip_serializing)]
    code: Vec<u8>,
    settings: Settings,
    initial_guesses: Vec<Vec<u8>>,
    initial_feedback: Vec<Feedback>,
    created_at: String,
    title: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateRequest {
    code: Vec<u8>,
    settings: Settings,
    #[serde(default)]
    initial_guesses: Vec<Vec<u8>>,
    #[serde(default)]
    title: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CheckRequest {
    guess: Vec<u8>,
}

#[derive(Debug, Serialize)]
struct RevealResponse {
    code: Vec<u8>,
}

#[derive(Debug, Serialize)]
struct ListResponse<'a> {
    problems: Vec<&'a Problem>,
}

#[derive(Clone)]
struct AppState {
    problems: Arc<RwLock<HashMap<String, Problem>>>,
}

// ──────────────────────────────────────────────────────────────
// Game logic
// ──────────────────────────────────────────────────────────────

fn evaluate_guess(guess: &[u8], code: &[u8]) -> Feedback {
    let mut blacks = 0usize;
    let mut code_rem: Vec<u8> = Vec::with_capacity(code.len());
    let mut guess_rem: Vec<u8> = Vec::with_capacity(code.len());
    for (g, c) in guess.iter().zip(code.iter()) {
        if g == c {
            blacks += 1;
        } else {
            code_rem.push(*c);
            guess_rem.push(*g);
        }
    }
    let mut whites = 0usize;
    for g in guess_rem {
        if let Some(pos) = code_rem.iter().position(|&c| c == g) {
            whites += 1;
            code_rem.swap_remove(pos);
        }
    }
    Feedback { blacks, whites }
}

fn validate_settings(s: &Settings) -> Result<(), String> {
    if !(2..=8).contains(&s.code_length) {
        return Err("codeLength must be in 2..=8".into());
    }
    if !(2..=8).contains(&s.num_colors) {
        return Err("numColors must be in 2..=8".into());
    }
    if !(1..=20).contains(&s.max_attempts) {
        return Err("maxAttempts must be in 1..=20".into());
    }
    if !s.allow_duplicates && s.num_colors < s.code_length {
        return Err("not enough colours for a duplicate-free code".into());
    }
    Ok(())
}

fn validate_row(row: &[u8], s: &Settings) -> Result<(), String> {
    if row.len() != s.code_length {
        return Err(format!(
            "row length {} does not match codeLength {}",
            row.len(),
            s.code_length
        ));
    }
    for &c in row {
        if (c as usize) >= s.num_colors {
            return Err(format!("colour {c} out of range 0..{}", s.num_colors));
        }
    }
    Ok(())
}

fn gen_id() -> String {
    const ALPHABET: &[u8] = b"23456789abcdefghjkmnpqrstuvwxyz";
    let mut rng = rand::thread_rng();
    (0..7)
        .map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char)
        .collect()
}

fn now_iso() -> String {
    // RFC 3339-ish without pulling chrono. Good enough for ordering.
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = d.as_secs() as i64;
    // Very minimal: emit as `YYYY-MM-DDTHH:MM:SSZ` using a tiny calendar calc.
    // For a small toy backend this is fine. (No leap-second handling.)
    let (y, mo, da, h, mi, s) = epoch_to_ymdhms(secs);
    format!("{y:04}-{mo:02}-{da:02}T{h:02}:{mi:02}:{s:02}Z")
}

fn epoch_to_ymdhms(secs: i64) -> (i32, u32, u32, u32, u32, u32) {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400) as u32;
    let h = rem / 3600;
    let mi = (rem % 3600) / 60;
    let s = rem % 60;
    let (y, mo, da) = days_to_ymd(days);
    (y, mo, da, h, mi, s)
}

fn days_to_ymd(days_since_epoch: i64) -> (i32, u32, u32) {
    // Algorithm by Howard Hinnant: civil_from_days.
    let z = days_since_epoch + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}

// ──────────────────────────────────────────────────────────────
// Error helper
// ──────────────────────────────────────────────────────────────

struct AppError(StatusCode, String);
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.0, self.1).into_response()
    }
}
impl<E: std::error::Error> From<E> for AppError {
    fn from(e: E) -> Self {
        AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    }
}
fn bad(msg: impl Into<String>) -> AppError {
    AppError(StatusCode::BAD_REQUEST, msg.into())
}
fn not_found() -> AppError {
    AppError(StatusCode::NOT_FOUND, "not found".into())
}

// ──────────────────────────────────────────────────────────────
// Handlers
// ──────────────────────────────────────────────────────────────

async fn create_problem(
    State(state): State<AppState>,
    Json(req): Json<CreateRequest>,
) -> Result<Json<Problem>, AppError> {
    validate_settings(&req.settings).map_err(bad)?;
    validate_row(&req.code, &req.settings).map_err(bad)?;
    if !req.settings.allow_duplicates {
        let mut sorted = req.code.clone();
        sorted.sort();
        sorted.dedup();
        if sorted.len() != req.code.len() {
            return Err(bad("duplicates not permitted in code"));
        }
    }
    if req.initial_guesses.len() >= req.settings.max_attempts {
        return Err(bad("too many initial guesses"));
    }
    for g in &req.initial_guesses {
        validate_row(g, &req.settings).map_err(bad)?;
    }

    let initial_feedback: Vec<Feedback> = req
        .initial_guesses
        .iter()
        .map(|g| evaluate_guess(g, &req.code))
        .collect();

    // None of the initial guesses may already solve the puzzle — that would
    // leave the solver nothing to do.
    if initial_feedback
        .iter()
        .any(|f| f.blacks == req.settings.code_length)
    {
        return Err(bad("an initial guess already solves the code"));
    }

    let id = gen_id();
    let problem = Problem {
        id: id.clone(),
        code: req.code,
        settings: req.settings,
        initial_guesses: req.initial_guesses,
        initial_feedback,
        created_at: now_iso(),
        title: req.title.and_then(|t| {
            let t = t.trim();
            if t.is_empty() { None } else { Some(t.to_string()) }
        }),
    };

    state
        .problems
        .write()
        .unwrap()
        .insert(id.clone(), problem.clone());
    Ok(Json(problem))
}

async fn list_problems(State(state): State<AppState>) -> Json<serde_json::Value> {
    let guard = state.problems.read().unwrap();
    let mut items: Vec<&Problem> = guard.values().collect();
    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Json(serde_json::json!({ "problems": items }))
}

async fn get_problem(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Problem>, AppError> {
    let guard = state.problems.read().unwrap();
    let p = guard.get(&id).ok_or_else(not_found)?;
    Ok(Json(p.clone()))
}

async fn check_guess(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<CheckRequest>,
) -> Result<Json<Feedback>, AppError> {
    let guard = state.problems.read().unwrap();
    let p = guard.get(&id).ok_or_else(not_found)?;
    validate_row(&req.guess, &p.settings).map_err(bad)?;
    Ok(Json(evaluate_guess(&req.guess, &p.code)))
}

async fn reveal_code(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<RevealResponse>, AppError> {
    let guard = state.problems.read().unwrap();
    let p = guard.get(&id).ok_or_else(not_found)?;
    Ok(Json(RevealResponse {
        code: p.code.clone(),
    }))
}

// ──────────────────────────────────────────────────────────────
// Main
// ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=info".into()),
        )
        .init();

    let state = AppState {
        problems: Arc::new(RwLock::new(HashMap::new())),
    };

    // Permissive CORS for development. Tighten for production.
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any)
        .allow_origin(Any);

    // Serve the static frontend from the project root (one level up).
    // Adjust if you deploy the frontend elsewhere.
    let static_dir = std::env::var("MASTERMIND_STATIC_DIR")
        .unwrap_or_else(|_| "..".to_string());
    let serve_dir = ServeDir::new(&static_dir).append_index_html_on_directories(true);

    let api = Router::new()
        .route("/problems", post(create_problem).get(list_problems))
        .route("/problems/:id", get(get_problem))
        .route("/problems/:id/check", post(check_guess))
        .route("/problems/:id/code", get(reveal_code))
        .with_state(state);

    let app = Router::new()
        .nest("/api", api)
        .fallback_service(serve_dir)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000);
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on http://{addr}  ·  static from {static_dir}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_correct_means_all_blacks() {
        let code = vec![0, 1, 2, 3];
        let fb = evaluate_guess(&code, &code);
        assert_eq!(fb.blacks, 4);
        assert_eq!(fb.whites, 0);
    }

    #[test]
    fn classic_two_black_one_white() {
        // code:  R G B Y
        // guess: R G Y X  -> 2 black (R,G), 1 white (Y misplaced), 0 for X
        let code = vec![0u8, 1, 2, 3];
        let guess = vec![0u8, 1, 3, 4];
        let fb = evaluate_guess(&guess, &code);
        assert_eq!(fb.blacks, 2);
        assert_eq!(fb.whites, 1);
    }

    #[test]
    fn duplicates_are_consumed_once() {
        // code:  R R G B
        // guess: R G R R  -> blacks: pos0 R==R. remaining code:[R,G,B] guess:[G,R,R]
        //                    whites: G matches G (consume), R matches one R (consume) -> 2 white
        let code = vec![0u8, 0, 1, 2];
        let guess = vec![0u8, 1, 0, 0];
        let fb = evaluate_guess(&guess, &code);
        assert_eq!(fb.blacks, 1);
        assert_eq!(fb.whites, 2);
    }

    #[test]
    fn id_is_seven_chars() {
        for _ in 0..50 {
            let id = gen_id();
            assert_eq!(id.len(), 7);
            assert!(id.chars().all(|c| c.is_ascii_alphanumeric()));
        }
    }
}

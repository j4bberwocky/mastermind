mod analysis;
mod api;
mod game;
mod state;

use axum::{
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use tower_http::{cors::CorsLayer, services::ServeDir, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use state::AppState;

#[tokio::main]
async fn main() {
    // Initialise structured logging (RUST_LOG controls the level)
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = AppState::new();

    let app = build_router(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(err) => {
            if err.kind() == std::io::ErrorKind::AddrInUse {
                tracing::error!(
                    "Could not start server on http://{}: address already in use. Stop the existing process or free port 8080 and try again.",
                    addr
                );
            } else {
                tracing::error!("Could not bind server to http://{}: {}", addr, err);
            }
            std::process::exit(1);
        }
    };

    tracing::info!("Mastermind server listening on http://{}", addr);

    if let Err(err) = axum::serve(listener, app).await {
        tracing::error!("Server error: {}", err);
        std::process::exit(1);
    }
}

/// Builds the application router (extracted so tests can reuse it).
pub fn build_router(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/api/health", get(health))
        // Game API
        .route("/api/game", post(api::create_game))
        .route("/api/game/:id", get(api::get_game))
        .route("/api/game/:id/guess", post(api::make_guess))
        .route("/api/game/:id/analyze", post(api::analyze))
        .route("/api/game/:id/export", get(api::export_game))
        // Serve the static frontend from the `static/` directory
        .fallback_service(ServeDir::new("static"))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok", "service": "mastermind" }))
}

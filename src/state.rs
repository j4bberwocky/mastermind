use dashmap::DashMap;
use std::sync::Arc;

use crate::game::Game;

/// Shared application state: a concurrent map of game ID → Game.
#[derive(Clone, Default)]
pub struct AppState {
    pub games: Arc<DashMap<String, Game>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            games: Arc::new(DashMap::new()),
        }
    }
}

// Package state provides thread-safe application state management.
package state

import (
	"sync"

	"github.com/j4bberwocky/mastermind/mastermind-go/internal/game"
)

// AppState holds the shared application state: a concurrent map of game ID → Game.
type AppState struct {
	mu    sync.RWMutex
	games map[string]*game.Game
}

// New creates a new AppState.
func New() *AppState {
	return &AppState{
		games: make(map[string]*game.Game),
	}
}

// Get retrieves a game by ID (returns a copy for safe read access).
func (s *AppState) Get(id string) (*game.Game, bool) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	g, ok := s.games[id]
	if !ok {
		return nil, false
	}
	// Return a copy to prevent races on read-only operations.
	cp := *g
	cp.Turns = make([]game.Turn, len(g.Turns))
	copy(cp.Turns, g.Turns)
	return &cp, true
}

// Set stores a game in the map.
func (s *AppState) Set(id string, g *game.Game) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.games[id] = g
}

// WithGame executes fn while holding the write lock on the game.
// Returns false if the game is not found.
func (s *AppState) WithGame(id string, fn func(g *game.Game)) bool {
	s.mu.Lock()
	defer s.mu.Unlock()
	g, ok := s.games[id]
	if !ok {
		return false
	}
	fn(g)
	return true
}

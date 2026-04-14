package state

import (
	"sync"
	"testing"

	"github.com/j4bberwocky/mastermind/mastermind-go/internal/game"
)

func TestSetAndGet(t *testing.T) {
	s := New()
	g := game.NewGame("test-1")
	s.Set("test-1", g)

	got, ok := s.Get("test-1")
	if !ok {
		t.Fatal("expected game to be found")
	}
	if got.ID != "test-1" {
		t.Errorf("got ID %s, want test-1", got.ID)
	}
}

func TestGetNotFound(t *testing.T) {
	s := New()
	_, ok := s.Get("nonexistent")
	if ok {
		t.Error("expected not found")
	}
}

func TestWithGame(t *testing.T) {
	s := New()
	g := game.NewGameWithSecret("test-1", game.Code{1, 2, 3, 4})
	s.Set("test-1", g)

	found := s.WithGame("test-1", func(g *game.Game) {
		_, _ = g.Guess(game.Code{1, 2, 3, 4})
	})
	if !found {
		t.Fatal("expected game to be found")
	}

	got, _ := s.Get("test-1")
	if got.Status != game.StatusWon {
		t.Errorf("expected Won status, got %s", got.Status)
	}
}

func TestWithGameNotFound(t *testing.T) {
	s := New()
	found := s.WithGame("nonexistent", func(g *game.Game) {
		t.Error("should not be called")
	})
	if found {
		t.Error("expected not found")
	}
}

func TestConcurrentAccess(t *testing.T) {
	s := New()
	const numGoroutines = 100

	// Pre-create some games
	for i := 0; i < 10; i++ {
		g := game.NewGame("game-" + string(rune('0'+i)))
		s.Set(g.ID, g)
	}

	var wg sync.WaitGroup
	wg.Add(numGoroutines)

	for i := 0; i < numGoroutines; i++ {
		go func(i int) {
			defer wg.Done()
			id := "concurrent-" + string(rune('a'+i%26))

			// Mix of writes and reads
			if i%2 == 0 {
				g := game.NewGame(id)
				s.Set(id, g)
			} else {
				s.Get(id)
			}
		}(i)
	}

	wg.Wait()
}

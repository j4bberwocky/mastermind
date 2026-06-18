## ADDED Requirements

### Requirement: Puzzles SHALL be rejected if not information-theoretically solvable

`POST /api/problems` SHALL reject puzzles for which the number of remaining attempts is smaller than the minimum number of guesses required, in the worst case, to distinguish the remaining candidate codes. Specifically, the request SHALL be rejected with `400 Bad Request` if:

```
R  <  ⌈ log_P(N) ⌉
```

where:

- `L` is `codeLength`
- `P = (L+1)(L+2)/2 - 1` is the count of valid feedback patterns
- `N` is the number of codes in the configured domain that produce the saved `initial_feedback` for every entry in `initial_guesses`
- `R = max_attempts - len(initial_guesses)` is the remaining attempts budget

#### Scenario: Trivially infeasible 4x6 with 2 attempts

- **WHEN** a client sends `POST /api/problems` with `codeLength=4`, `numColors=6`, `allowDuplicates=true`, `maxAttempts=2`, no initial guesses
- **THEN** the server SHALL respond `400 Bad Request` with a message indicating at least 3 attempts are required

#### Scenario: Solvable 4x6 classic accepted

- **WHEN** a client sends `POST /api/problems` with `codeLength=4`, `numColors=6`, `allowDuplicates=true`, `maxAttempts=10`, no initial guesses, and a valid code
- **THEN** the server SHALL respond `200 OK` and persist the puzzle

#### Scenario: Initial guesses reduce candidate space enough to make a low-attempt puzzle feasible

- **WHEN** a puzzle has initial guesses that narrow the candidate set to a small `N` such that `R ≥ ⌈log_P(N)⌉`
- **THEN** the server SHALL accept the puzzle even if `R` would have been insufficient without the initial guesses

#### Scenario: 8x8 with insufficient attempts

- **WHEN** a client sends `POST /api/problems` with `codeLength=8`, `numColors=8`, `allowDuplicates=true`, `maxAttempts=4`, no initial guesses
- **THEN** the server SHALL respond `400 Bad Request` (`8^8 ≈ 16.7M` candidates need at least 5 attempts)

### Requirement: Candidate enumeration SHALL be exact

The backend SHALL count candidates by enumerating the full code domain and checking consistency with the saved initial feedback exhaustively, rather than using upper-bound heuristics. The domain is the Cartesian product `numColors^codeLength` when `allowDuplicates=true`, or the set of permutations `P(numColors, codeLength)` when `allowDuplicates=false`.

#### Scenario: allowDuplicates=true uses Cartesian product

- **WHEN** the backend counts candidates for `codeLength=4`, `numColors=6`, `allowDuplicates=true`, no initial guesses
- **THEN** `N` SHALL equal exactly `6^4 = 1296`

#### Scenario: allowDuplicates=false uses permutations

- **WHEN** the backend counts candidates for `codeLength=4`, `numColors=6`, `allowDuplicates=false`, no initial guesses
- **THEN** `N` SHALL equal exactly `P(6,4) = 6*5*4*3 = 360`

#### Scenario: Initial guesses with non-empty feedback narrow the candidate set

- **WHEN** the backend counts candidates with one initial guess whose computed feedback is `{blacks: 1, whites: 1}` for `codeLength=4`, `numColors=6`, `allowDuplicates=true`
- **THEN** `N` SHALL be strictly less than `1296`, equal to the exact count of codes producing that feedback against the guess

### Requirement: Validity error message SHALL be actionable

When a puzzle is rejected for infeasibility, the `400 Bad Request` response body SHALL include a human-readable message identifying both the minimum number of attempts required and the number the user supplied, so the frontend can display it directly to the puzzle author.

#### Scenario: Error message content

- **WHEN** a puzzle is rejected for infeasibility with `min_attempts_needed=5` and `R=3`
- **THEN** the response body SHALL contain text equivalent to "Puzzle non risolvibile nel caso peggiore: servono almeno 5 tentativi, ne hai 3"

### Requirement: Validity check SHALL run on every POST but NOT on PATCH

Because `PATCH /api/problems/:id` only updates the title (not code/settings/initial guesses), the validity check SHALL NOT be re-evaluated on PATCH. It SHALL run on every `POST /api/problems` as a gate before persistence.

#### Scenario: PATCH does not re-validate

- **WHEN** a client renames a puzzle via `PATCH /api/problems/:id`
- **THEN** the server SHALL NOT re-run the candidate enumeration

#### Scenario: POST always validates

- **WHEN** any `POST /api/problems` is received
- **THEN** the server SHALL compute `N` and `⌈log_P(N)⌉` before responding success and SHALL persist the puzzle only if `R ≥ ⌈log_P(N)⌉`

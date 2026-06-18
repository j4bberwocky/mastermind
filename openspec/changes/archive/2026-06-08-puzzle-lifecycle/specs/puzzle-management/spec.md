## ADDED Requirements

### Requirement: User SHALL be able to rename a puzzle's title

The backend SHALL expose `PATCH /api/problems/:id` accepting a JSON body `{ "title": string | null }` that updates only the `title` field of the addressed puzzle. The server SHALL trim whitespace; an empty or whitespace-only string SHALL be stored as NULL.

#### Scenario: Successful rename

- **WHEN** a client sends `PATCH /api/problems/abc1234` with body `{"title": "Pasqua 2026"}`
- **THEN** the server SHALL return `200 OK` with the public view of the puzzle reflecting the new title, and subsequent `GET /api/problems/abc1234` SHALL return the new title

#### Scenario: Clearing the title

- **WHEN** a client sends `PATCH /api/problems/abc1234` with body `{"title": ""}` (or `"   "`)
- **THEN** the server SHALL store the title as NULL and the response SHALL include `"title": null`

#### Scenario: Patch on unknown id

- **WHEN** a client sends `PATCH /api/problems/nope` for a non-existent id
- **THEN** the server SHALL return `404 Not Found`

#### Scenario: Title too long

- **WHEN** a client sends `PATCH /api/problems/abc1234` with a title longer than 80 characters
- **THEN** the server SHALL return `400 Bad Request` with a message indicating the length limit, and the existing title SHALL remain unchanged

### Requirement: User SHALL be able to delete a puzzle

The backend SHALL expose `DELETE /api/problems/:id` that permanently removes the puzzle. Deletion SHALL be hard (no soft-delete, no recovery).

#### Scenario: Successful delete

- **WHEN** a client sends `DELETE /api/problems/abc1234` for an existing puzzle
- **THEN** the server SHALL return `204 No Content`, and subsequent `GET /api/problems/abc1234` SHALL return `404`, and the puzzle SHALL NOT appear in `GET /api/problems`

#### Scenario: Delete on unknown id

- **WHEN** a client sends `DELETE /api/problems/nope` for a non-existent id
- **THEN** the server SHALL return `404 Not Found`

### Requirement: Code, settings, and initial guesses SHALL be immutable after creation

The backend SHALL NOT accept changes to `code`, `codeLength`, `numColors`, `allowDuplicates`, `maxAttempts`, or `initialGuesses` of an existing puzzle through any endpoint. Only `title` is mutable.

#### Scenario: PATCH attempts to modify code

- **WHEN** a client sends `PATCH /api/problems/abc1234` with a body containing fields other than `title` (e.g., `{"code": [0,1,2,3]}`)
- **THEN** the server SHALL either ignore the unknown fields and apply only the title (if present) or return `400 Bad Request`; in no case SHALL the puzzle's code be modified

### Requirement: Title length SHALL be validated server-side

The backend SHALL enforce that titles are at most 80 characters at both the request-validation layer (returning `400` to the client) and at the database level (CHECK constraint as defense in depth).

#### Scenario: Create with overlong title

- **WHEN** a client sends `POST /api/problems` with a title longer than 80 characters
- **THEN** the server SHALL return `400 Bad Request` and the puzzle SHALL NOT be persisted

### Requirement: Delete UI SHALL require explicit confirmation

The frontend BrowsePage SHALL display a delete control (e.g., a trash icon) on each puzzle card that, when activated, triggers a modal confirmation dialog with two clearly labeled actions ("Cancella" / "Annulla"). Only confirming the destructive action SHALL invoke the `DELETE` request.

#### Scenario: Tap on trash icon

- **WHEN** the user taps the trash icon on a puzzle card
- **THEN** a confirmation modal SHALL appear, and no `DELETE` request SHALL be issued

#### Scenario: Confirmation

- **WHEN** the user confirms deletion in the modal
- **THEN** the frontend SHALL issue `DELETE /api/problems/:id` and remove the card from the list on success

#### Scenario: Cancel

- **WHEN** the user cancels the modal (Annulla button, Esc key, or backdrop click)
- **THEN** the modal SHALL close and the puzzle SHALL remain unchanged

### Requirement: Rename UI SHALL support inline editing without modal

The frontend BrowsePage SHALL allow renaming a puzzle via inline edit-in-place (e.g., a pencil icon next to the title). Submitting (Enter) SHALL issue `PATCH`; canceling (Esc or focus loss without changes) SHALL discard the edit.

#### Scenario: Successful inline rename

- **WHEN** the user activates rename, edits the title, and presses Enter
- **THEN** the frontend SHALL issue `PATCH /api/problems/:id` with the new title and update the card on success

#### Scenario: Cancel inline rename with Esc

- **WHEN** the user activates rename, edits the title, and presses Esc
- **THEN** no `PATCH` request SHALL be issued and the displayed title SHALL revert to the original

### Requirement: Edit and delete controls SHALL be hidden in mock mode

When the frontend is operating in `localStorage` mock mode (no live backend), the rename and delete controls SHALL NOT be displayed on puzzle cards, because the mock store is documented as a throw-away fallback.

#### Scenario: Mock mode hides controls

- **WHEN** the frontend determines the backend is unreachable and falls back to mock mode
- **THEN** puzzle cards in BrowsePage SHALL NOT show rename or delete controls

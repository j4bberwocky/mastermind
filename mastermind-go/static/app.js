/* ── State ─────────────────────────────────────────────────────────────── */
let gameId = null;
let currentGuess = [0, 0, 0, 0]; // 0 = empty
let isStartingGame = false;
let isSubmittingGuess = false;
const CODE_LENGTH = 4;
const NUM_COLORS = 6;
const MAX_ATTEMPTS = 10;

const COLOR_NAMES = {
  0: 'Empty', 1: 'Red', 2: 'Orange', 3: 'Yellow',
  4: 'Green', 5: 'Blue', 6: 'Purple'
};

/* ── DOM helpers ───────────────────────────────────────────────────────── */
const $ = id => document.getElementById(id);
const show = id => $(id).classList.remove('hidden');
const hide = id => $(id).classList.add('hidden');

function showMessage(text, type = '') {
  const el = $('message-banner');
  el.textContent = text;
  el.className = 'message-banner' + (type ? ` ${type}` : '');
  show('message-banner');
}

async function readErrorMessage(res, fallback) {
  try {
    const data = await res.json();
    return data.error || fallback;
  } catch {
    try {
      const text = await res.text();
      return text || fallback;
    } catch {
      return fallback;
    }
  }
}

function setNewGameDisabled(disabled) {
  $('btn-new-game').disabled = disabled;
}

function setGuessControlsDisabled(disabled) {
  const pegButtons = $('guess-input').querySelectorAll('button.peg');
  pegButtons.forEach(btn => {
    btn.disabled = disabled;
  });

  $('btn-submit').disabled = disabled;
}

function pegHtml(color, size = '') {
  const cls = color > 0 ? `peg peg-${color}` : 'peg peg-empty';
  const sizeAttr = size ? ` style="width:${size};height:${size};"` : '';
  return `<span class="${cls}"${sizeAttr} title="${COLOR_NAMES[color]}"></span>`;
}

/* ── Cycle peg color on click ──────────────────────────────────────────── */
function cyclePeg(pos) {
  if (!gameId || isSubmittingGuess) return;
  currentGuess[pos] = (currentGuess[pos] % NUM_COLORS) + 1;
  updateGuessInput();
}

function updateGuessInput() {
  const buttons = $('guess-input').querySelectorAll('button.peg');
  buttons.forEach((btn, i) => {
    const v = currentGuess[i];
    btn.className = `peg ${v > 0 ? `peg-${v}` : 'peg-empty'}`;
    btn.title = COLOR_NAMES[v] || 'Empty';
  });
}

/* ── Start a new game ──────────────────────────────────────────────────── */
$('btn-new-game').addEventListener('click', async () => {
  if (isStartingGame) return;

  try {
    isStartingGame = true;
    setNewGameDisabled(true);

    const res = await fetch('/api/game', { method: 'POST' });
    if (!res.ok) {
      throw new Error(await readErrorMessage(res, 'Unable to start a new game.'));
    }

    const data = await res.json();

    gameId = data.game_id;
    currentGuess = [0, 0, 0, 0];
    isSubmittingGuess = false;

    // Reset UI
    $('board-body').innerHTML = '';
    hide('end-section');
    hide('analysis-section');
    hide('message-banner');

    $('info-attempts').textContent = `Attempts: 0 / ${MAX_ATTEMPTS}`;
    $('info-status').textContent = 'In Progress';
    $('info-status').className = 'badge badge-inprogress';

    updateGuessInput();
  setGuessControlsDisabled(false);
    show('game-info');
    show('legend');
    show('guess-section');
    show('board-section');
    showMessage(data.message);
  } catch (err) {
    showMessage('Failed to start game: ' + err.message, 'error');
  } finally {
    isStartingGame = false;
    setNewGameDisabled(false);
  }
});

/* ── Submit a guess ────────────────────────────────────────────────────── */
async function submitGuess() {
  if (!gameId || isSubmittingGuess) return;

  if (currentGuess.includes(0)) {
    showMessage('Please fill all 4 positions before submitting.', 'error');
    return;
  }

  try {
    isSubmittingGuess = true;
    setGuessControlsDisabled(true);
    const submittedGuess = [...currentGuess];

    const res = await fetch(`/api/game/${gameId}/guess`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ guess: submittedGuess }),
    });

    if (!res.ok) {
      showMessage(await readErrorMessage(res, 'Error submitting guess'), 'error');
      return;
    }

    const data = await res.json();
    appendTurnToBoard(data.attempt, submittedGuess, data.blacks, data.whites);
    $('info-attempts').textContent = `Attempts: ${data.attempt} / ${MAX_ATTEMPTS}`;

    if (data.status === 'won' || data.status === 'lost') {
      handleGameOver(data);
    } else {
      showMessage(data.message);
      // Reset guess
      currentGuess = [0, 0, 0, 0];
      updateGuessInput();
    }
  } catch (err) {
    showMessage('Network error: ' + err.message, 'error');
  } finally {
    isSubmittingGuess = false;
    if ($('guess-section').classList.contains('hidden')) {
      setGuessControlsDisabled(true);
    } else {
      setGuessControlsDisabled(false);
    }
  }
}

/* ── Append a turn row to the board ────────────────────────────────────── */
function appendTurnToBoard(attempt, guess, blacks, whites) {
  const tbody = $('board-body');
  const row = document.createElement('tr');

  const pegCells = guess
    .map(v => `<td><div class="cell-peg">${pegHtml(v)}</div></td>`)
    .join('');

  row.innerHTML = `
    <td>${attempt}</td>
    ${pegCells}
    <td><span class="blacks-val">${blacks}</span></td>
    <td><span class="whites-val">${whites}</span></td>
  `;
  tbody.appendChild(row);
}

/* ── Handle game over ──────────────────────────────────────────────────── */
function handleGameOver(data) {
  hide('guess-section');

  const statusBadge = $('info-status');
  if (data.status === 'won') {
    $('end-title').textContent = '🎉 You Win!';
    statusBadge.textContent = 'Won';
    statusBadge.className = 'badge badge-won';
    showMessage(data.message, 'success');
  } else {
    $('end-title').textContent = '😞 Game Over';
    statusBadge.textContent = 'Lost';
    statusBadge.className = 'badge badge-lost';
    showMessage(data.message, 'error');
  }

  // Show secret
  const secretDisplay = $('secret-display');
  secretDisplay.innerHTML = (data.secret || []).map(v => pegHtml(v)).join('');

  show('end-section');
}

/* ── Analyze the game ──────────────────────────────────────────────────── */
async function analyzeGame() {
  if (!gameId) return;
  $('btn-analyze').textContent = '⏳ Analyzing…';
  $('btn-analyze').disabled = true;

  try {
    const res = await fetch(`/api/game/${gameId}/analyze`, { method: 'POST' });
    if (!res.ok) {
      showMessage(await readErrorMessage(res, 'Analysis failed'), 'error');
      return;
    }
    const analysis = await res.json();
    renderAnalysis(analysis);
    show('analysis-section');
    $('analysis-section').scrollIntoView({ behavior: 'smooth' });
  } catch (err) {
    showMessage('Analysis error: ' + err.message, 'error');
  } finally {
    $('btn-analyze').textContent = '🔍 Analyze My Game';
    $('btn-analyze').disabled = false;
  }
}

/* ── Render analysis results ────────────────────────────────────────────── */
function renderAnalysis(a) {
  const summaryEl = $('analysis-summary');
  const score = a.optimality_score;
  const scoreColor = score >= 80 ? '#2ecc71' : score >= 50 ? '#f39c12' : '#e74c3c';

  summaryEl.innerHTML = `
    <p>${escHtml(a.summary)}</p>
    <p style="margin-top:0.5rem;font-size:0.9rem;color:var(--muted);">
      Optimality score: <strong style="color:${scoreColor}">${score}/100</strong>
    </p>
    <div class="score-bar-wrap">
      <div class="score-bar" style="width:${score}%;background:${scoreColor};"></div>
    </div>
  `;

  const turnsEl = $('analysis-turns');
  turnsEl.innerHTML = a.turns.map(t => {
    const optimal = t.was_optimal;
    const cardClass = optimal ? 'turn-card optimal' : 'turn-card suboptimal';
    const badgeClass = optimal ? 'turn-badge optimal' : 'turn-badge suboptimal';
    const badgeText = optimal ? '✅ Optimal' : '⚠️ Suboptimal';

    const guessPegs = t.guess.map(v => pegHtml(v, '28px')).join(' ');
    const candidatesInfo = `
      <span>Candidates before: <strong>${t.candidates_before}</strong></span>
      <span>→ after: <strong>${t.candidates_after}</strong></span>
      <span>Worst-case remaining: <strong>${t.actual_worst_case}</strong></span>
    `;

    let suggestionHtml = '';
    if (!optimal && t.suggested_guess) {
      const sugPegs = t.suggested_guess.map(v => pegHtml(v, '28px')).join(' ');
      suggestionHtml = `
        <div class="turn-detail-row" style="margin-top:0.5rem;color:var(--warning);">
          💡 Suggested: ${sugPegs}
          <span>(worst-case: <strong>${t.suggested_worst_case}</strong>)</span>
        </div>`;
    }

    return `
      <div class="${cardClass}">
        <div class="turn-card-header">
          <span class="turn-card-title">Attempt #${t.attempt}</span>
          <span class="${badgeClass}">${badgeText}</span>
        </div>
        <div class="turn-detail-row">${guessPegs}
          <span style="margin-left:0.5rem;">→ ⚫ ${t.feedback.blacks} &nbsp; ⚪ ${t.feedback.whites}</span>
        </div>
        <div class="turn-details" style="margin-top:0.5rem;">
          <div class="turn-detail-row">${candidatesInfo}</div>
          ${suggestionHtml}
        </div>
      </div>`;
  }).join('');
}

/* ── Security helper ───────────────────────────────────────────────────── */
function escHtml(str) {
  return String(str)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

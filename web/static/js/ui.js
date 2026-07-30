import { renderBoard, getIntersection } from './board.js';
import { createPieceElement, getCachedPieceNames } from './pieces.js';
import { showMoveHints, clearHints, setSelected } from './hints.js';
import { postHints, getRules } from './api.js';

let gameState = null;
let selection = { type: null };
let onAction = null; /* callback to main: (action) => void */

export function init(actionCallback) {
    onAction = actionCallback;
    bindToolbar();
    bindBoard();
    bindPool();
    bindOverlay();
    bindGameOver();
}

function bindToolbar() {
    document.getElementById('btn-new').addEventListener('click', () => {
        if (onAction) onAction({ type: 'new_game', board: { width: 9, height: 10 } });
    });
    document.getElementById('btn-undo').addEventListener('click', () => {
        if (onAction) onAction({ type: 'undo' });
    });
    document.getElementById('btn-custom').addEventListener('click', openCustomPanel);
    document.getElementById('btn-rules').addEventListener('click', openRulesPanel);
    document.getElementById('btn-pass').addEventListener('click', () => {
        if (onAction) onAction({ type: 'pass' });
    });
    document.getElementById('btn-resign').addEventListener('click', () => {
        if (onAction) onAction({ type: 'resign' });
    });
}

function bindBoard() {
    document.getElementById('board').addEventListener('click', (e) => {
        const intn = e.target.closest('.intersection');
        if (!intn) return;
        const x = Number(intn.dataset.x);
        const y = Number(intn.dataset.y);
        handleBoardClick(x, y);
    });
}

function bindPool() {
    document.getElementById('red-pool').addEventListener('click', (e) => {
        const pieceEl = e.target.closest('.piece');
        if (!pieceEl) return;
        handlePoolClick(pieceEl.dataset.pieceName, pieceEl.dataset.pieceColor);
    });
    document.getElementById('black-pool').addEventListener('click', (e) => {
        const pieceEl = e.target.closest('.piece');
        if (!pieceEl) return;
        handlePoolClick(pieceEl.dataset.pieceName, pieceEl.dataset.pieceColor);
    });
}

function bindOverlay() {
    document.getElementById('overlay').addEventListener('click', (e) => {
        if (e.target === e.currentTarget) closeAllPanels();
    });
    for (const btn of document.querySelectorAll('.panel-close')) {
        btn.addEventListener('click', closeAllPanels);
    }

    /* tabs */
    document.querySelector('.panel-tabs').addEventListener('click', (e) => {
        if (!e.target.classList.contains('tab')) return;
        const tabName = e.target.dataset.tab;
        for (const t of document.querySelectorAll('.panel-tabs .tab')) {
            t.classList.toggle('active', t === e.target);
        }
        for (const c of document.querySelectorAll('.panel .tab-content')) {
            c.classList.toggle('active', c.id === tabName);
        }
    });

    /* custom panel buttons */
    document.getElementById('btn-size-confirm').addEventListener('click', confirmSizeGame);
    document.getElementById('btn-random-confirm').addEventListener('click', confirmRandomGame);
    document.getElementById('btn-load-confirm').addEventListener('click', confirmLoadGame);

    /* popup */
    document.getElementById('popup-choose').addEventListener('click', (e) => {
        const btn = e.target.closest('button');
        if (!btn) return;
        handlePopupChoice(btn.dataset.choice);
    });
}

function bindGameOver() {
    document.getElementById('btn-game-over-new').addEventListener('click', () => {
        hideGameOver();
        if (onAction) onAction({ type: 'new_game', board: { width: 9, height: 10 } });
    });
    document.getElementById('btn-game-over-close').addEventListener('click', hideGameOver);
}

/* ======== State & Render ======== */

function phase() {
    if (!gameState) return 'placement';
    return (gameState.red_pool.length > 0 || gameState.black_pool.length > 0) ? 'placement' : 'movement';
}

export function render(state) {
    gameState = state;
    clearSelection();
    renderBoard(state);
    renderPools(state);
    renderToolbar(state);

    const p = phase();
    const playing = state.result === 'Unfinished' && p === 'movement';
    document.getElementById('btn-pass').style.display = playing ? '' : 'none';
    document.getElementById('btn-resign').style.display = state.result === 'Unfinished' ? '' : 'none';
    document.getElementById('btn-undo').disabled = !state.can_undo;
    document.getElementById('sidebar').style.display = p === 'placement' ? '' : 'none';

    if (state.result !== 'Unfinished') {
        setStatus(resultLabel(state.result));
        showGameOver(state.result);
    } else if (p === 'placement') {
        hideGameOver();
        setStatus(`${playerLabel(state.player)}方布子`);
    } else {
        hideGameOver();
        setStatus(`${playerLabel(state.player)}方行棋`);
    }
}

function renderPools(state) {
    const redItems = document.getElementById('red-pool').querySelector('.pool-items');
    const blackItems = document.getElementById('black-pool').querySelector('.pool-items');

    redItems.innerHTML = '';
    blackItems.innerHTML = '';

    for (const piece of state.red_pool) {
        redItems.appendChild(createPieceElement(piece, true));
    }
    for (const piece of state.black_pool) {
        blackItems.appendChild(createPieceElement(piece, true));
    }
}

function playerLabel(player) {
    return player === 'Red' ? '红' : '黑';
}

function resultLabel(result) {
    switch (result) {
        case 'RedWin': return '红胜';
        case 'BlackWin': return '黑胜';
        case 'Draw': return '和棋';
        default: return '';
    }
}

function renderToolbar(state) {
    document.getElementById('player-indicator').textContent = `行棋方：${playerLabel(state.player)}`;
    document.getElementById('white-indicator').textContent = `白子×${state.white_pool}`;
}

let statusTimeout = null;
export function setStatus(msg, error = false) {
    clearTimeout(statusTimeout);
    const el = document.getElementById('status');
    el.textContent = msg;
    el.classList.toggle('error', error);
    if (!error && msg) {
        statusTimeout = setTimeout(() => { el.textContent = ''; }, 4000);
    }
}

function clearSelection() {
    selection = { type: null };
    clearHints();
    for (const el of document.querySelectorAll('.intersection.selected')) {
        el.classList.remove('selected');
    }
    for (const el of document.querySelectorAll('.pool-piece-selected')) {
        el.classList.remove('pool-piece-selected');
    }
}

/* ======== Board Click Handling ======== */

async function handleBoardClick(x, y) {
    const p = phase();
    const intn = getIntersection(x, y);
    const hasPiece = intn && intn.querySelector('.piece');
    const hintType = intn ? (intn.dataset.hintType || '') : '';
    const hintTypes = intn ? (intn.dataset.hintTypes || '') : '';

    /* Phase: movement — clicking a move/capture/push/draw/leave target */
    if (p === 'movement' && (hintType === 'move' || hintType === 'capture' || hintType === 'push' || hintType === 'draw' || hintType === 'leave' || hintTypes)) {
        if (hintTypes) {
            showPopup(x, y, hintTypes.split(','));
        } else {
            executeHintAction(hintType, x, y);
        }
        return;
    }

    /* Phase: placement — clicking own half */
    if (p === 'placement' && selection.type === 'pool_piece' && !hasPiece) {
        if (isOwnHalf(x, y, selection.piece.color)) {
            if (onAction) onAction({ type: 'place', piece: { name: selection.piece.name, color: selection.piece.color }, to: [x, y] });
            return;
        }
        setStatus('只能放在己方半区', true);
        return;
    }

    /* Phase: movement — clicking own piece to query hints */
    if (p === 'movement' && hasPiece && !hintType && !hintTypes) {
        clearSelection();
        setSelected(x, y);
        try {
            const hints = await postHints({ x, y });
            showMoveHints(hints.moves);
            if (hints.moves && hints.moves.length === 0) {
                setStatus('该棋子无可行动作', true);
            }
        } catch (e) {
            setStatus(e.message, true);
        }
        return;
    }

    /* anything else: clear selection */
    clearSelection();
}

function isOwnHalf(x, y, color) {
    if (!gameState) return false;
    const h = gameState.board.height;
    if (color === 'Red') return y >= Math.ceil(h / 2);
    return y < Math.floor(h / 2);
}

function executeHintAction(hintType, x, y) {
    if (hintType === 'move') {
        if (onAction) onAction({ type: 'move', from: getSelectedBoardPos(), to: [x, y] });
    } else if (hintType === 'capture') {
        if (onAction) onAction({ type: 'capture', from: getSelectedBoardPos(), to: [x, y] });
    } else if (hintType === 'push') {
        if (onAction) onAction({ type: 'push', from: getSelectedBoardPos(), to: [x, y] });
    } else if (hintType === 'draw') {
        if (onAction) onAction({ type: 'draw', from: getSelectedBoardPos(), to: [x, y] });
    } else if (hintType === 'leave') {
        if (onAction) onAction({ type: 'leave', from: getSelectedBoardPos(), to: [x, y] });
    } else {
        console.error('unknown hint type for action:', hintType);
    }
}

function getSelectedBoardPos() {
    const sel = document.querySelector('.intersection.selected');
    if (sel) return [Number(sel.dataset.x), Number(sel.dataset.y)];
    return [0, 0];
}

/* ======== Pool Click Handling ======== */

function handlePoolClick(name, color) {
    if (phase() !== 'placement') return;
    if (gameState && color !== gameState.player) return;

    clearSelection();
    selection = { type: 'pool_piece', piece: { name, color } };

    const el = document.querySelector(`.pool .piece[data-piece-name="${name}"][data-piece-color="${color}"]`);
    if (el && el.parentElement) el.parentElement.classList.add('pool-piece-selected');
}

/* ======== Popup (capture / push / draw / leave choice) ======== */

function showPopup(x, y, types) {
    const popup = document.getElementById('popup-choose');
    popup.innerHTML = '';
    for (const t of types) {
        const btn = document.createElement('button');
        btn.dataset.choice = t;
        btn.dataset.tx = x;
        btn.dataset.ty = y;
        if (t === 'move') {
            btn.textContent = '移动';
            btn.className = 'move-opt';
        } else if (t === 'draw') {
            btn.textContent = '和棋';
            btn.className = 'draw-opt';
        } else if (t === 'capture') {
            btn.textContent = '捉子';
            btn.className = 'capture-opt';
        } else if (t === 'push') {
            btn.textContent = '推子';
            btn.className = 'push-opt';
        } else if (t === 'leave') {
            btn.textContent = '留守';
            btn.className = 'leave-opt';
        } else {
            console.error('unknown hint type for popup:', t);
        }
        popup.appendChild(btn);
    }

    const intn = getIntersection(x, y);
    if (intn) {
        const rect = intn.getBoundingClientRect();
        popup.style.left = `${rect.right + 4}px`;
        popup.style.top = `${rect.top}px`;
    }

    popup.classList.remove('hidden');
}

function handlePopupChoice(actionType) {
    const popup = document.getElementById('popup-choose');
    const btn = popup.querySelector(`button[data-choice="${actionType}"]`);
    const x = Number(btn.dataset.tx);
    const y = Number(btn.dataset.ty);
    popup.classList.add('hidden');

    if (onAction) onAction({ type: actionType, from: getSelectedBoardPos(), to: [x, y] });
}

export function hidePopup() {
    document.getElementById('popup-choose').classList.add('hidden');
}

/* ======== Custom Panel ======== */

function openCustomPanel() {
    document.getElementById('overlay').classList.remove('hidden');
    document.getElementById('panel-custom').classList.remove('hidden');
    document.getElementById('panel-rules').classList.add('hidden');
}

function confirmSizeGame() {
    const w = clampSize(Number(document.getElementById('custom-width').value), 1, 16);
    const h = clampSize(Number(document.getElementById('custom-height').value), 1, 16);
    closeAllPanels();
    if (onAction) onAction({ type: 'new_game', board: { width: w, height: h } });
}

function confirmRandomGame() {
    const w = clampSize(Number(document.getElementById('rand-width').value), 1, 16);
    const h = clampSize(Number(document.getElementById('rand-height').value), 2, 16);

    const redSlots = w * (h - Math.ceil(h / 2));
    const blackSlots = w * Math.floor(h / 2);
    if (redSlots < 16 || blackSlots < 16) {
        setStatus(`棋盘太小，每方半区至少需要 16 个位置（当前红 ${redSlots} / 黑 ${blackSlots}）`, true);
        return;
    }

    closeAllPanels();

    if (onAction) {
        const randomConfig = buildRandomConfig(w, h);
        onAction({ type: 'new_game', ...randomConfig });
    }
}

function confirmLoadGame() {
    const text = document.getElementById('load-text').value.trim();
    if (!text) return;
    closeAllPanels();

    if (onAction) {
        onAction({ type: 'new_game', notation: text });
    }
}

function closeAllPanels() {
    document.getElementById('overlay').classList.add('hidden');
    document.getElementById('panel-custom').classList.add('hidden');
    document.getElementById('panel-rules').classList.add('hidden');
}

function clampSize(v, min, max) {
    return Math.max(min, Math.min(max, Number.isFinite(v) ? v : min));
}

/* ======== Random Layout ======== */

function buildRandomConfig(width, height) {
    const half = Math.floor(height / 2);
    const midpoint = Math.ceil(height / 2);

    const redPositions = [];
    const blackPositions = [];
    for (let x = 0; x < width; x++) {
        for (let y = midpoint; y < height; y++) redPositions.push([x, y]);
        for (let y = 0; y < half; y++) blackPositions.push([x, y]);
    }

    shuffle(redPositions);
    shuffle(blackPositions);

    const cells = Array.from({ length: height }, () => Array.from({ length: width }, () => null));

    const pieceNames = getCachedPieceNames();
    for (let i = 0; i < pieceNames.length; i++) {
        const [rx, ry] = redPositions[i];
        cells[ry][rx] = { name: pieceNames[i], color: 'Red' };
    }
    for (let i = 0; i < pieceNames.length; i++) {
        const [bx, by] = blackPositions[i];
        cells[by][bx] = { name: pieceNames[i], color: 'Black' };
    }

    return {
        board: { width, height, cells },
        red_pool: [],
        black_pool: [],
        white_pool: 0,
        player: 'Red',
    };
}

function shuffle(arr) {
    for (let i = arr.length - 1; i > 0; i--) {
        const j = Math.floor(Math.random() * (i + 1));
        [arr[i], arr[j]] = [arr[j], arr[i]];
    }
    return arr;
}

/* ======== Rules Panel ======== */

async function openRulesPanel() {
    document.getElementById('overlay').classList.remove('hidden');
    document.getElementById('panel-rules').classList.remove('hidden');
    document.getElementById('panel-custom').classList.add('hidden');

    const content = document.getElementById('panel-rules').querySelector('.rules-content');
    if (content.dataset.loaded === '1') return;
    content.dataset.loaded = '1';

    try {
        const data = await getRules();
        content.textContent = data.text || '';
    } catch (e) {
        content.textContent = '无法加载规则';
    }
}

/* ======== Game Over Overlay ======== */

function showGameOver(result) {
    const overlay = document.getElementById('game-over');
    const title = overlay.querySelector('.game-over-title');
    title.textContent = resultLabel(result);
    title.className = 'game-over-title';
    switch (result) {
        case 'RedWin': title.classList.add('red'); break;
        case 'BlackWin': title.classList.add('black'); break;
        case 'Draw': title.classList.add('draw'); break;
    }
    overlay.classList.remove('hidden');
}

function hideGameOver() {
    document.getElementById('game-over').classList.add('hidden');
}

/* ======== Export for main.js ======== */

export function isPlacementPhase() {
    return phase() === 'placement';
}

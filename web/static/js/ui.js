import { renderBoard, getIntersection } from './board.js';
import { createPieceElement } from './pieces.js';
import {
    showMoveHints,
    clearHints,
    clearSelection as clearBoardSelection,
    setSelected,
    previewAction,
    clearCandidatePreview,
    showPlayedAction as highlightPlayedAction,
    clearPlayedAction,
} from './hints.js';
import { postLegalActions, getRules } from './api.js';

let gameState = null;
let selection = { type: null };
let onCommand = null;
let busy = false;
let analysisCandidates = [];
let selectedCandidateIndex = null;

export function init(commandCallback) {
    onCommand = commandCallback;
    bindToolbar();
    bindBoard();
    bindPool();
    bindAnalysis();
    bindOverlay();
    bindGameOver();
}

function bindToolbar() {
    document.getElementById('btn-new').addEventListener('click', () => {
        sendCommand({
            type: 'new_game',
            board: { width: 9, height: 10 },
            controllers: controllerSettings(),
        });
    });
    document.getElementById('btn-undo').addEventListener('click', () => {
        sendCommand({ type: 'undo' });
    });
    document.getElementById('btn-custom').addEventListener('click', openCustomPanel);
    document.getElementById('btn-rules').addEventListener('click', openRulesPanel);
    document.getElementById('btn-agent-hint').addEventListener('click', () => {
        sendCommand({ type: 'agent_analyze' });
    });
    document.getElementById('btn-agent-step').addEventListener('click', () => {
        sendCommand({ type: 'agent_step' });
    });
    document.getElementById('btn-pass').addEventListener('click', () => {
        submitAction({ type: 'pass' });
    });
    document.getElementById('btn-resign').addEventListener('click', () => {
        submitAction({ type: 'resign' });
    });
}

function bindBoard() {
    document.getElementById('board').addEventListener('click', (event) => {
        const intersection = event.target.closest('.intersection');
        if (!intersection) return;
        const x = Number(intersection.dataset.x);
        const y = Number(intersection.dataset.y);
        handleBoardClick(x, y);
    });
}

function bindPool() {
    document.getElementById('red-pool').addEventListener('click', (event) => {
        const piece = event.target.closest('.piece');
        if (!piece) return;
        handlePoolClick(piece.dataset.pieceName, piece.dataset.pieceColor);
    });
    document.getElementById('black-pool').addEventListener('click', (event) => {
        const piece = event.target.closest('.piece');
        if (!piece) return;
        handlePoolClick(piece.dataset.pieceName, piece.dataset.pieceColor);
    });
}

function bindAnalysis() {
    document.getElementById('analysis-candidates').addEventListener('click', (event) => {
        const row = event.target.closest('.candidate-row');
        if (!row) return;
        selectCandidate(Number(row.dataset.index));
    });
    document.getElementById('btn-apply-candidate').addEventListener('click', () => {
        if (selectedCandidateIndex === null || !canHumanAct()) return;
        const candidate = analysisCandidates[selectedCandidateIndex];
        if (candidate) submitAction(candidate.action);
    });
}

function bindOverlay() {
    document.getElementById('overlay').addEventListener('click', (event) => {
        if (event.target === event.currentTarget) closeAllPanels();
    });
    for (const button of document.querySelectorAll('.panel-close')) {
        button.addEventListener('click', closeAllPanels);
    }

    document.querySelector('.panel-tabs').addEventListener('click', (event) => {
        if (!event.target.classList.contains('tab')) return;
        const tabName = event.target.dataset.tab;
        for (const tab of document.querySelectorAll('.panel-tabs .tab')) {
            tab.classList.toggle('active', tab === event.target);
        }
        for (const content of document.querySelectorAll('.panel .tab-content')) {
            content.classList.toggle('active', content.id === tabName);
        }
    });

    document.getElementById('btn-size-confirm').addEventListener('click', confirmSizeGame);
    document.getElementById('btn-random-confirm').addEventListener('click', confirmRandomGame);
    document.getElementById('btn-load-confirm').addEventListener('click', confirmLoadGame);

    document.getElementById('popup-choose').addEventListener('click', (event) => {
        const button = event.target.closest('button');
        if (!button) return;
        handlePopupChoice(button.dataset.choice);
    });
}

function bindGameOver() {
    document.getElementById('btn-game-over-new').addEventListener('click', () => {
        hideGameOver();
        sendCommand({
            type: 'new_game',
            board: { width: 9, height: 10 },
            controllers: controllerSettings(),
        });
    });
    document.getElementById('btn-game-over-close').addEventListener('click', hideGameOver);
}

function sendCommand(command) {
    if (!busy && onCommand) onCommand(command);
}

function submitAction(action) {
    if (!canHumanAct()) return;
    sendCommand({ type: 'submit_action', action });
}

function phase() {
    return gameState ? gameState.phase : 'placement';
}

function canHumanAct() {
    return Boolean(gameState && gameState.can_human_act && !busy);
}

export function render(state) {
    gameState = state;
    clearInteraction();
    clearAnalysis();
    renderBoard(state);
    renderPools(state);
    renderToolbar(state);
    syncControllerSettings(state);

    const placement = phase() === 'placement';
    document.getElementById('sidebar').classList.toggle('hidden', !placement);

    if (state.result !== 'Unfinished') {
        setStatus(resultLabel(state.result));
        showGameOver(state.result);
    } else {
        hideGameOver();
        const verb = placement ? '布子' : '行棋';
        setStatus(playerLabel(state.player) + '方（' + controlLabel(state.current_controller.control) + '）' + verb);
    }
}

export function showPlayedAction(action) {
    clearCandidatePreview();
    highlightPlayedAction(action);
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

function controlLabel(control) {
    return control === 'agent' ? 'AI' : '人类';
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
    const current = state.current_controller;
    const controller = current.control === 'agent'
        ? 'AI（' + current.agent + '）'
        : '人类';
    document.getElementById('player-indicator').textContent =
        '行棋方：' + playerLabel(state.player) + ' · ' + controller;
    document.getElementById('white-indicator').textContent = '白子×' + state.white_pool;
    refreshInteractivity();
}

function refreshInteractivity() {
    if (!gameState) return;
    const unfinished = gameState.result === 'Unfinished';
    const movement = phase() === 'movement';
    const human = canHumanAct();
    const canStep = unfinished && gameState.can_agent_step;

    document.getElementById('btn-new').disabled = busy;
    document.getElementById('btn-custom').disabled = busy;
    document.getElementById('btn-agent-hint').disabled = busy || !unfinished;
    document.getElementById('btn-agent-step').classList.toggle('hidden', !canStep);
    document.getElementById('btn-agent-step').disabled = busy || !canStep;
    document.getElementById('btn-pass').classList.toggle('hidden', !movement || !unfinished);
    document.getElementById('btn-pass').disabled = !human;
    document.getElementById('btn-resign').classList.toggle('hidden', !unfinished);
    document.getElementById('btn-resign').disabled = !human;
    document.getElementById('btn-undo').disabled = busy || !gameState.can_undo;
    document.getElementById('board-wrap').classList.toggle('interaction-locked', !human);
    document.getElementById('sidebar').classList.toggle('interaction-locked', !human);
    updateApplyButton();
}

export function setBusy(value) {
    busy = value;
    document.getElementById('app').classList.toggle('is-busy', busy);
    refreshInteractivity();
}

let statusTimeout = null;
export function setStatus(message, error = false) {
    clearTimeout(statusTimeout);
    const element = document.getElementById('status');
    element.textContent = message;
    element.classList.toggle('error', error);
    if (!error && message) {
        statusTimeout = setTimeout(() => { element.textContent = ''; }, 4000);
    }
}

function clearInteraction() {
    selection = { type: null };
    clearHints();
    clearBoardSelection();
    clearPlayedAction();
    hidePopup();
    for (const element of document.querySelectorAll('.pool-piece-selected')) {
        element.classList.remove('pool-piece-selected');
    }
}

function clearManualSelectionForPreview() {
    selection = { type: null };
    clearHints();
    clearBoardSelection();
    clearPlayedAction();
    hidePopup();
    for (const element of document.querySelectorAll('.pool-piece-selected')) {
        element.classList.remove('pool-piece-selected');
    }
}

async function handleBoardClick(x, y) {
    if (!canHumanAct()) return;
    clearAnalysisSelection();

    const currentPhase = phase();
    const intersection = getIntersection(x, y);
    const hasPiece = intersection && intersection.querySelector('.piece');
    const hintType = intersection ? (intersection.dataset.hintType || '') : '';
    const hintTypes = intersection ? (intersection.dataset.hintTypes || '') : '';

    if (currentPhase === 'movement' && (hintType || hintTypes)) {
        if (hintTypes) {
            showPopup(x, y, hintTypes.split(','));
        } else {
            executeHintAction(hintType, x, y);
        }
        return;
    }

    if (currentPhase === 'placement' && selection.type === 'pool_piece' && !hasPiece) {
        if (isOwnHalf(y, selection.piece.color)) {
            submitAction({
                type: 'place',
                piece: { name: selection.piece.name, color: selection.piece.color },
                to: [x, y],
            });
            return;
        }
        setStatus('只能放在己方半区', true);
        return;
    }

    if (currentPhase === 'movement' && hasPiece && !hintType && !hintTypes) {
        clearInteraction();
        setSelected(x, y);
        try {
            const response = await postLegalActions({
                revision: gameState.revision,
                side: gameState.player,
                from: [x, y],
            });
            if (!gameState || response.revision !== gameState.revision) return;
            showMoveHints(response.actions);
            if (response.actions.length === 0) {
                setStatus('该棋子无可行动作', true);
            }
        } catch (error) {
            setStatus(error.message, true);
        }
        return;
    }

    clearInteraction();
}

function isOwnHalf(y, color) {
    if (!gameState) return false;
    const height = gameState.board.height;
    if (color === 'Red') return y >= Math.ceil(height / 2);
    return y < Math.floor(height / 2);
}

function executeHintAction(actionType, x, y) {
    submitAction({ type: actionType, from: selectedBoardPosition(), to: [x, y] });
}

function selectedBoardPosition() {
    const selected = document.querySelector('.intersection.selected');
    if (!selected) return [0, 0];
    return [Number(selected.dataset.x), Number(selected.dataset.y)];
}

function handlePoolClick(name, color) {
    if (!canHumanAct() || phase() !== 'placement') return;
    if (color !== gameState.player) return;

    clearAnalysisSelection();
    clearInteraction();
    selection = { type: 'pool_piece', piece: { name, color } };

    const selector = '.pool .piece[data-piece-name="' + name + '"][data-piece-color="' + color + '"]';
    const element = document.querySelector(selector);
    if (element && element.parentElement) {
        element.parentElement.classList.add('pool-piece-selected');
    }
}

function showPopup(x, y, types) {
    const popup = document.getElementById('popup-choose');
    popup.innerHTML = '';
    for (const type of types) {
        const button = document.createElement('button');
        button.dataset.choice = type;
        button.dataset.tx = x;
        button.dataset.ty = y;
        const option = actionOption(type);
        button.textContent = option.label;
        button.className = option.className;
        popup.appendChild(button);
    }

    const intersection = getIntersection(x, y);
    if (intersection) {
        const rect = intersection.getBoundingClientRect();
        popup.style.left = (rect.right + 4) + 'px';
        popup.style.top = rect.top + 'px';
    }
    popup.classList.remove('hidden');
}

function actionOption(type) {
    switch (type) {
        case 'move': return { label: '移动', className: 'move-opt' };
        case 'draw': return { label: '和棋', className: 'draw-opt' };
        case 'capture': return { label: '捉子', className: 'capture-opt' };
        case 'push': return { label: '推子', className: 'push-opt' };
        case 'divide': return { label: '分兵', className: 'divide-opt' };
        default: return { label: type, className: '' };
    }
}

function handlePopupChoice(actionType) {
    const popup = document.getElementById('popup-choose');
    const button = popup.querySelector('button[data-choice="' + actionType + '"]');
    if (!button) return;
    const x = Number(button.dataset.tx);
    const y = Number(button.dataset.ty);
    hidePopup();
    executeHintAction(actionType, x, y);
}

export function hidePopup() {
    document.getElementById('popup-choose').classList.add('hidden');
}

export function showAnalysis(response) {
    if (!gameState || response.revision !== gameState.revision) return;
    analysisCandidates = response.candidates || [];
    selectedCandidateIndex = null;
    clearCandidatePreview();

    const meta = document.getElementById('analysis-meta');
    meta.textContent = response.agent + ' · ' + analysisCandidates.length + ' 个候选';
    renderAnalysisCandidates();
}

function renderAnalysisCandidates() {
    const container = document.getElementById('analysis-candidates');
    container.innerHTML = '';

    if (analysisCandidates.length === 0) {
        const empty = document.createElement('div');
        empty.className = 'analysis-empty';
        empty.textContent = '当前没有候选。';
        container.appendChild(empty);
    } else {
        analysisCandidates.forEach((candidate, index) => {
            const row = document.createElement('button');
            row.type = 'button';
            row.className = 'candidate-row';
            row.dataset.index = index;
            const rank = document.createElement('span');
            rank.className = 'candidate-rank';
            rank.textContent = String(index + 1);

            const notation = document.createElement('span');
            notation.className = 'candidate-notation';
            notation.textContent = candidate.notation;

            const score = document.createElement('span');
            score.className = 'candidate-score';
            score.textContent = formatScore(candidate.score);

            row.append(rank, notation, score);
            container.appendChild(row);
        });
    }
    updateApplyButton();
}

function formatScore(score) {
    return Number(score).toLocaleString('zh-CN', { maximumFractionDigits: 3 });
}

function selectCandidate(index) {
    const candidate = analysisCandidates[index];
    if (!candidate) return;
    clearManualSelectionForPreview();
    selectedCandidateIndex = index;

    for (const row of document.querySelectorAll('.candidate-row')) {
        row.classList.toggle('selected', Number(row.dataset.index) === index);
    }
    previewAction(candidate.action);
    updateApplyButton();
}

function clearAnalysisSelection() {
    selectedCandidateIndex = null;
    clearCandidatePreview();
    for (const row of document.querySelectorAll('.candidate-row.selected')) {
        row.classList.remove('selected');
    }
    updateApplyButton();
}

export function clearAnalysis() {
    analysisCandidates = [];
    selectedCandidateIndex = null;
    clearCandidatePreview();
    document.getElementById('analysis-meta').textContent = '点击“AI 提示”获取当前局面的候选。';
    document.getElementById('analysis-candidates').innerHTML = '';
    updateApplyButton();
}

function updateApplyButton() {
    const button = document.getElementById('btn-apply-candidate');
    const human = Boolean(gameState && gameState.can_human_act);
    button.classList.toggle('hidden', !human);
    button.disabled = busy || !human || selectedCandidateIndex === null;
}

function openCustomPanel() {
    if (busy) return;
    document.getElementById('overlay').classList.remove('hidden');
    document.getElementById('panel-custom').classList.remove('hidden');
    document.getElementById('panel-rules').classList.add('hidden');
}

function confirmSizeGame() {
    const width = clampSize(Number(document.getElementById('custom-width').value), 1, 16);
    const height = clampSize(Number(document.getElementById('custom-height').value), 1, 16);
    closeAllPanels();
    sendCommand({
        type: 'new_game',
        board: { width, height },
        controllers: controllerSettings(),
    });
}

function confirmRandomGame() {
    const width = clampSize(Number(document.getElementById('rand-width').value), 1, 16);
    const height = clampSize(Number(document.getElementById('rand-height').value), 2, 16);
    const redSlots = width * (height - Math.ceil(height / 2));
    const blackSlots = width * Math.floor(height / 2);
    if (redSlots < 16 || blackSlots < 16) {
        setStatus(
            '棋盘太小，每方半区至少需要 16 个位置（当前红 ' +
                redSlots + ' / 黑 ' + blackSlots + '）',
            true,
        );
        return;
    }

    closeAllPanels();
    sendCommand({
        type: 'new_game',
        board: { width, height },
        controllers: controllerSettings(),
        random_placement: true,
    });
}

function confirmLoadGame() {
    const notation = document.getElementById('load-text').value.trim();
    if (!notation) return;
    closeAllPanels();
    sendCommand({
        type: 'new_game',
        notation,
        controllers: controllerSettings(),
    });
}

function controllerSettings() {
    return {
        red: document.getElementById('red-control').value,
        black: document.getElementById('black-control').value,
    };
}

function syncControllerSettings(state) {
    document.getElementById('red-control').value = state.controllers.red.control;
    document.getElementById('black-control').value = state.controllers.black.control;
}

function closeAllPanels() {
    document.getElementById('overlay').classList.add('hidden');
    document.getElementById('panel-custom').classList.add('hidden');
    document.getElementById('panel-rules').classList.add('hidden');
}

function clampSize(value, min, max) {
    return Math.max(min, Math.min(max, Number.isFinite(value) ? value : min));
}

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
    } catch (error) {
        content.textContent = '无法加载规则：' + error.message;
    }
}

function showGameOver(result) {
    const overlay = document.getElementById('game-over');
    const title = overlay.querySelector('.game-over-title');
    title.textContent = resultLabel(result);
    title.className = 'game-over-title';
    if (result === 'RedWin') title.classList.add('red');
    if (result === 'BlackWin') title.classList.add('black');
    if (result === 'Draw') title.classList.add('draw');
    overlay.classList.remove('hidden');
}

function hideGameOver() {
    document.getElementById('game-over').classList.add('hidden');
}

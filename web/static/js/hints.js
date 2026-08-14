import { getIntersection } from './board.js';

const ACTION_LABELS = {
    place: '布子',
    move: '移动',
    draw: '和棋',
    capture: '捉子',
    push: '推子',
    pull: '拉子',
    resign: '认负',
    switch: '切换',
};

function uniqueTypes(types) {
    return [...new Set(types)];
}

function actionLabel(type) {
    return ACTION_LABELS[type] || type;
}

function addCorners(marker) {
    for (const position of ['top-left', 'top-right', 'bottom-left', 'bottom-right']) {
        const corner = document.createElement('span');
        corner.className = `action-marker-corner corner-${position}`;
        marker.appendChild(corner);
    }
}

function addOriginMarker(intersection, state) {
    const marker = document.createElement('div');
    marker.className = `action-marker action-origin marker-${state}`;
    addCorners(marker);
    intersection.appendChild(marker);
}

function addTargetMarker(intersection, state) {
    const marker = document.createElement('div');
    marker.className = `action-marker action-target marker-${state}`;
    addCorners(marker);
    intersection.appendChild(marker);
}

function addTooltip(intersection, text, state) {
    if (!text) return;

    let tooltip = intersection.querySelector('.board-tooltip');
    if (!tooltip) {
        tooltip = document.createElement('div');
        tooltip.className = 'board-tooltip';
        tooltip.setAttribute('role', 'tooltip');
        intersection.appendChild(tooltip);
    }

    const action = document.createElement('div');
    action.className = `board-tooltip-action tooltip-${state}`;
    action.textContent = text;
    tooltip.prepend(action);
}

function removeElements(selector) {
    for (const element of document.querySelectorAll(selector)) {
        element.remove();
    }
    for (const tooltip of document.querySelectorAll('.board-tooltip:empty')) {
        tooltip.remove();
    }
}

export function showMoveHints(actions, switchablePositions = []) {
    clearHints();
    if (!actions) return;

    const byPosition = {};
    const actionTargets = new Set();
    for (const action of actions) {
        if (!action.to) continue;
        const key = `${action.to[0]},${action.to[1]}`;
        if (!byPosition[key]) byPosition[key] = [];
        byPosition[key].push(action.type);
        actionTargets.add(key);
    }
    for (const [x, y] of switchablePositions) {
        const key = `${x},${y}`;
        if (!byPosition[key]) byPosition[key] = [];
        byPosition[key].push('switch');
    }

    for (const [key, actionTypes] of Object.entries(byPosition)) {
        const [x, y] = key.split(',').map(Number);
        const intersection = getIntersection(x, y);
        if (!intersection) continue;

        const types = uniqueTypes(actionTypes);
        const multiple = types.length > 1;
        if (actionTargets.has(key)) intersection.classList.add('hint-target');
        if (types.includes('switch')) intersection.classList.add('hint-switch');
        if (multiple) intersection.classList.add('hint-multi');

        if (multiple) {
            intersection.dataset.hintTypes = types.join(',');
        } else {
            intersection.dataset.hintType = types[0];
        }

        if (actionTargets.has(key)) addTargetMarker(intersection, 'legal');
        const labels = types.map(actionLabel);
        addTooltip(intersection, multiple ? `可选：${labels.join(' · ')}` : labels[0], 'legal');
    }
}

export function clearHints() {
    for (const element of document.querySelectorAll(
        '.intersection.hint-target, .intersection.hint-switch, .intersection.hint-multi',
    )) {
        element.classList.remove('hint-target', 'hint-switch', 'hint-multi');
        delete element.dataset.hintType;
        delete element.dataset.hintTypes;
    }
    removeElements('.marker-legal, .tooltip-legal');
}

export function clearSelection() {
    for (const element of document.querySelectorAll('.intersection.selected')) {
        element.classList.remove('selected');
    }
    removeElements('.marker-selected');
}

export function setSelected(x, y) {
    clearSelection();
    clearHints();
    const intersection = getIntersection(x, y);
    if (!intersection) return;
    intersection.classList.add('selected');
    addOriginMarker(intersection, 'selected');
}

export function previewAction(action) {
    clearCandidatePreview();
    const origin = action.from || action.at;
    if (origin) {
        const from = getIntersection(origin[0], origin[1]);
        if (from) {
            from.classList.add('candidate-from');
            addOriginMarker(from, 'candidate');
            if (!action.to) {
                addTooltip(from, `AI 推荐：${actionLabel(action.type)}`, 'candidate');
            }
        }
    }
    if (action.to) {
        const to = getIntersection(action.to[0], action.to[1]);
        if (to) {
            to.classList.add('candidate-to');
            addTargetMarker(to, 'candidate');
            addTooltip(to, `AI 推荐：${actionLabel(action.type)}`, 'candidate');
        }
    }
}

export function clearCandidatePreview() {
    for (const element of document.querySelectorAll('.candidate-from, .candidate-to')) {
        element.classList.remove('candidate-from', 'candidate-to');
    }
    removeElements('.marker-candidate, .tooltip-candidate');
}

export function showPlayedAction(action) {
    clearPlayedAction();
    const origin = action.from || action.at;
    if (origin) {
        const from = getIntersection(origin[0], origin[1]);
        if (from) {
            from.classList.add('played-from');
            addOriginMarker(from, 'played');
            if (!action.to) {
                addTooltip(from, `上一步：${actionLabel(action.type)}`, 'played');
            }
        }
    }
    if (action.to) {
        const to = getIntersection(action.to[0], action.to[1]);
        if (to) {
            to.classList.add('played-to');
            addTargetMarker(to, 'played');
            addTooltip(to, `上一步：${actionLabel(action.type)}`, 'played');
        }
    }
}

export function clearPlayedAction() {
    for (const element of document.querySelectorAll('.played-from, .played-to')) {
        element.classList.remove('played-from', 'played-to');
    }
    removeElements('.marker-played, .tooltip-played');
}

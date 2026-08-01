import { getIntersection } from './board.js';

export function showMoveHints(actions) {
    clearHints();
    if (!actions) return;

    const byPosition = {};
    for (const action of actions) {
        if (!action.to) continue;
        const key = `${action.to[0]},${action.to[1]}`;
        if (!byPosition[key]) byPosition[key] = [];
        byPosition[key].push(action.type);
    }

    for (const [key, types] of Object.entries(byPosition)) {
        const [x, y] = key.split(',').map(Number);
        const intersection = getIntersection(x, y);
        if (!intersection) continue;

        let className;
        if (types.length > 1) {
            className = 'hint-multi';
        } else if (types.includes('draw')) {
            className = 'hint-draw';
        } else if (types.includes('capture')) {
            className = 'hint-capture';
        } else if (types.includes('push')) {
            className = 'hint-push';
        } else if (types.includes('divide')) {
            className = 'hint-divide';
        } else {
            className = 'hint-move';
        }
        intersection.classList.add(className);

        if (types.length > 1) {
            intersection.dataset.hintTypes = types.join(',');
        } else {
            intersection.dataset.hintType = types[0];
        }
    }
}

export function clearHints() {
    for (const element of document.querySelectorAll('.intersection')) {
        element.classList.remove(
            'hint-move',
            'hint-capture',
            'hint-push',
            'hint-draw',
            'hint-multi',
            'hint-divide',
        );
        delete element.dataset.hintType;
        delete element.dataset.hintTypes;
    }
}

export function clearSelection() {
    for (const element of document.querySelectorAll('.intersection.selected')) {
        element.classList.remove('selected');
    }
}

export function setSelected(x, y) {
    clearSelection();
    clearHints();
    const intersection = getIntersection(x, y);
    if (intersection) intersection.classList.add('selected');
}

export function previewAction(action) {
    clearCandidatePreview();
    if (action.from) {
        const from = getIntersection(action.from[0], action.from[1]);
        if (from) from.classList.add('candidate-from');
    }
    if (action.to) {
        const to = getIntersection(action.to[0], action.to[1]);
        if (to) to.classList.add('candidate-to');
    }
}

export function clearCandidatePreview() {
    for (const element of document.querySelectorAll('.candidate-from, .candidate-to')) {
        element.classList.remove('candidate-from', 'candidate-to');
    }
}

export function showPlayedAction(action) {
    clearPlayedAction();
    if (action.from) {
        const from = getIntersection(action.from[0], action.from[1]);
        if (from) from.classList.add('played-from');
    }
    if (action.to) {
        const to = getIntersection(action.to[0], action.to[1]);
        if (to) to.classList.add('played-to');
    }
}

export function clearPlayedAction() {
    for (const element of document.querySelectorAll('.played-from, .played-to')) {
        element.classList.remove('played-from', 'played-to');
    }
}

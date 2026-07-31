import { getIntersection } from './board.js';

export function showMoveHints(moves) {
    clearHints();
    if (!moves) return;

    const byPos = {};
    for (const m of moves) {
        const key = `${m.to[0]},${m.to[1]}`;
        if (!byPos[key]) byPos[key] = [];
        byPos[key].push(m.type);
    }

    for (const [key, types] of Object.entries(byPos)) {
        const [xs, ys] = key.split(',');
        const intn = getIntersection(Number(xs), Number(ys));
        if (!intn) continue;

        let cls;
        if (types.length > 1) {
            cls = 'hint-multi';
        } else if (types.includes('draw')) {
            cls = 'hint-draw';
        } else if (types.includes('capture')) {
            cls = 'hint-capture';
        } else if (types.includes('push')) {
            cls = 'hint-push';
        } else if (types.includes('divide')) {
            cls = 'hint-divide';
        } else {
            cls = 'hint-move';
        }
        intn.classList.add(cls);

        if (types.length > 1) {
            intn.dataset.hintTypes = types.join(',');
        } else {
            intn.dataset.hintType = types[0];
        }
    }
}

export function clearHints() {
    for (const el of document.querySelectorAll('.intersection')) {
        el.classList.remove('hint-move', 'hint-capture', 'hint-push', 'hint-draw', 'hint-multi', 'hint-divide');
        delete el.dataset.hintType;
        delete el.dataset.hintTypes;
    }
}

export function clearSelection() {
    for (const el of document.querySelectorAll('.intersection.selected')) {
        el.classList.remove('selected');
    }
}

export function setSelected(x, y) {
    clearSelection();
    clearHints();
    const intn = getIntersection(x, y);
    if (intn) intn.classList.add('selected');
}

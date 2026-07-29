export function getShapeClass(formation) {
    switch (formation) {
        case 165: return 'shape-square';
        case 90:  return 'shape-diamond';
        case 162: return 'shape-triangle-up';
        case 69:  return 'shape-triangle-down';
        default:   return 'shape-circle';
    }
}

export function createPieceElement(piece, pool = false) {
    const el = document.createElement('div');
    el.className = `piece piece-${piece.color.toLowerCase()} ${getShapeClass(piece.formation)}`;

    const text = document.createElement('span');
    text.className = 'piece-text';
    text.textContent = piece.name;
    el.appendChild(text);

    if (pool) {
        el.dataset.pieceName = piece.name;
        el.dataset.pieceColor = piece.color;

        const wrap = document.createElement('div');
        wrap.className = 'piece-wrap';
        wrap.appendChild(el);
        return wrap;
    }

    return el;
}

let cachedPieceNames = null;

export function cachePieceListFromState(state) {
    if (state.red_pool && state.red_pool.length > 0) {
        cachedPieceNames = state.red_pool.map(p => p.name);
    }
}

export function getCachedPieceNames() {
    return cachedPieceNames || [];
}

export function getShapeClass(formation) {
    switch (formation) {
        case 165: return 'shape-square';
        case 90: return 'shape-diamond';
        case 162: return 'shape-triangle-up';
        case 69: return 'shape-triangle-down';
        default: return 'shape-circle';
    }
}

export function createPieceElement(piece, pool = false) {
    const element = document.createElement('div');
    element.className = `piece piece-${piece.color.toLowerCase()} ${getShapeClass(piece.formation)}`;

    const text = document.createElement('span');
    text.className = 'piece-text';
    text.textContent = piece.name;
    element.appendChild(text);

    if (!pool) return element;

    element.dataset.pieceName = piece.name;
    element.dataset.pieceColor = piece.color;

    const wrap = document.createElement('div');
    wrap.className = 'piece-wrap';
    wrap.appendChild(element);
    return wrap;
}

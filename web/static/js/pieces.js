const SVG_NS = 'http://www.w3.org/2000/svg';

export function getShapeClass(formation) {
    switch (formation) {
        case 165: return 'shape-square';
        case 90: return 'shape-diamond';
        case 162: return 'shape-triangle-up';
        case 69: return 'shape-triangle-down';
        case 93: return 'shape-pentagon-down';
        case 186: return 'shape-pentagon-up';
        default: return 'shape-circle';
    }
}

function createShapeElement(shapeClass) {
    const svg = document.createElementNS(SVG_NS, 'svg');
    svg.classList.add('piece-shape');
    svg.setAttribute('viewBox', '0 0 100 100');
    svg.setAttribute('aria-hidden', 'true');
    svg.setAttribute('focusable', 'false');

    let shape;
    if (shapeClass === 'shape-square') {
        shape = document.createElementNS(SVG_NS, 'rect');
        shape.setAttribute('x', '6');
        shape.setAttribute('y', '6');
        shape.setAttribute('width', '88');
        shape.setAttribute('height', '88');
        shape.setAttribute('rx', '2');
    } else if (shapeClass === 'shape-circle') {
        shape = document.createElementNS(SVG_NS, 'circle');
        shape.setAttribute('cx', '50');
        shape.setAttribute('cy', '50');
        shape.setAttribute('r', '44');
    } else {
        shape = document.createElementNS(SVG_NS, 'polygon');
        const points = {
            'shape-diamond': '50,6 94,50 50,94 6,50',
            'shape-triangle-up': '50,6 94,94 6,94',
            'shape-triangle-down': '6,6 94,6 50,94',
            'shape-pentagon-up': '5,95 95,95 95,50 50,5 5,50',
            'shape-pentagon-down': '5,5 95,5 95,50 50,95 5,50',
        };
        shape.setAttribute('points', points[shapeClass]);
    }
    shape.classList.add('piece-shape-surface');
    svg.appendChild(shape);
    return svg;
}

export function createPieceElement(piece, pool = false) {
    const element = document.createElement('div');
    const shapeClass = getShapeClass(piece.formation);
    element.className = `piece piece-${piece.player.toLowerCase()} ${shapeClass}`;
    element.appendChild(createShapeElement(shapeClass));

    const text = document.createElement('span');
    text.className = 'piece-text';
    text.textContent = piece.name;
    element.appendChild(text);

    if (!pool) return element;

    element.dataset.pieceName = piece.name;
    element.dataset.piecePlayer = piece.player;

    const wrap = document.createElement('div');
    wrap.className = 'piece-wrap';
    wrap.appendChild(element);

    const tooltip = createPoolPieceTooltip(piece);
    wrap.appendChild(tooltip);
    wrap.addEventListener('mouseenter', () => {
        document.body.appendChild(tooltip);
        tooltip.classList.add('visible');
        positionPoolPieceTooltip(wrap, tooltip);
    });
    wrap.addEventListener('mouseleave', () => {
        tooltip.classList.remove('visible');
        wrap.appendChild(tooltip);
    });
    return wrap;
}

function positionPoolPieceTooltip(piece, tooltip) {
    const pieceRect = piece.getBoundingClientRect();
    const tooltipRect = tooltip.getBoundingClientRect();
    const gap = 6;
    const left = Math.min(
        Math.max(8, pieceRect.left + pieceRect.width / 2 - tooltipRect.width / 2),
        window.innerWidth - tooltipRect.width - 8,
    );
    const above = pieceRect.top - tooltipRect.height - gap;
    const top = above >= 8 ? above : pieceRect.bottom + gap;
    tooltip.style.left = `${left}px`;
    tooltip.style.top = `${top}px`;
}

function createPoolPieceTooltip(piece) {
    const tooltip = document.createElement('div');
    tooltip.className = 'pool-piece-tooltip';
    tooltip.setAttribute('role', 'tooltip');

    const title = document.createElement('div');
    title.className = 'pool-piece-tooltip-title';
    title.textContent = '初始能力';
    tooltip.appendChild(title);

    const list = document.createElement('div');
    list.className = 'pool-piece-tooltip-list';
    for (const ability of piece.abilities || []) {
        const item = document.createElement('span');
        item.className = ability.effective ? 'effective' : 'ineffective';
        item.textContent = ability.name;
        list.appendChild(item);
    }
    tooltip.appendChild(list);
    return tooltip;
}

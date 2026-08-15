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
    bindPoolTooltip(wrap, tooltip);
    return wrap;
}

function showPoolPieceTooltip(wrap, tooltip) {
    document.body.appendChild(tooltip);
    tooltip.classList.add('visible');
    positionPoolPieceTooltip(wrap, tooltip);
}

function hidePoolPieceTooltip(wrap, tooltip) {
    tooltip.classList.remove('visible');
    wrap.appendChild(tooltip);
}

function bindPoolTooltip(wrap, tooltip) {
    if (window.matchMedia('(hover: hover)').matches) {
        wrap.addEventListener('mouseenter', () => showPoolPieceTooltip(wrap, tooltip));
        wrap.addEventListener('mouseleave', () => hidePoolPieceTooltip(wrap, tooltip));
        return;
    }

    const LONG_PRESS_MS = 500;
    const MOVE_TOLERANCE = 10;
    let timer = null;
    let startX = 0;
    let startY = 0;
    let longPressed = false;

    function cancel() {
        if (timer) {
            clearTimeout(timer);
            timer = null;
        }
        hidePoolPieceTooltip(wrap, tooltip);
    }

    // 长按触发后，阻止随后冒泡的 click 触发棋池选中
    wrap.addEventListener('click', (event) => {
        if (!longPressed) return;
        event.preventDefault();
        event.stopPropagation();
        longPressed = false;
    }, true);

    wrap.addEventListener('touchstart', (event) => {
        longPressed = false;
        cancel();
        if (event.touches.length !== 1) return;
        const touch = event.touches[0];
        startX = touch.clientX;
        startY = touch.clientY;
        timer = setTimeout(() => {
            longPressed = true;
            showPoolPieceTooltip(wrap, tooltip);
        }, LONG_PRESS_MS);
    }, { passive: true });

    wrap.addEventListener('touchmove', (event) => {
        if (!timer) return;
        const touch = event.touches[0];
        if (!touch ||
            Math.abs(touch.clientX - startX) > MOVE_TOLERANCE ||
            Math.abs(touch.clientY - startY) > MOVE_TOLERANCE) {
            cancel();
            longPressed = false;
        }
    }, { passive: true });

    wrap.addEventListener('touchend', cancel);
    wrap.addEventListener('touchcancel', cancel);
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
    appendFormationEffect(tooltip, piece);
    return tooltip;
}

export function appendFormationEffect(tooltip, piece) {
    const effect = piece.formation_effect;
    if (!effect) return;

    const formation = document.createElement('div');
    formation.className = 'formation-effect';

    const title = document.createElement('div');
    title.className = 'formation-effect-title';
    title.textContent = '阵法效果';
    formation.appendChild(title);

    formation.appendChild(formationEffectLine('对己方：', effect.allies));
    formation.appendChild(formationEffectLine('对敌方：', effect.enemies));

    tooltip.appendChild(formation);
}

function formationEffectLine(prefix, effectLine) {
    const line = document.createElement('div');
    line.className = 'formation-effect-line';
    line.appendChild(document.createTextNode(prefix));

    const grants = (effectLine && effectLine.grants) || [];
    const strips = (effectLine && effectLine.strips) || [];

    if (grants.length === 0 && strips.length === 0) {
        line.appendChild(document.createTextNode('无'));
        return line;
    }
    if (grants.length > 0) {
        const grant = document.createElement('span');
        grant.className = 'formation-grant';
        grant.textContent = '赋予 ' + grants.join('、');
        line.appendChild(grant);
    }
    if (strips.length > 0) {
        const strip = document.createElement('span');
        strip.className = 'formation-strip';
        strip.textContent = '剥夺 ' + strips.join('、');
        line.appendChild(strip);
    }
    return line;
}

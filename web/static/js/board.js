import { createPieceElement } from './pieces.js';

const SVG_NS = 'http://www.w3.org/2000/svg';
const boardEl = document.getElementById('board');
const NUMERALS = ['零', '一', '二', '三', '四', '五', '六', '七', '八', '九', '十', '甲', '乙', '丙', '丁', '戊', '己', '庚', '辛', '壬', '癸'];

function numeral(n) {
    return NUMERALS[n] ?? '?';
}

function cellSize(cols) {
    const base = parseInt(getComputedStyle(document.documentElement).getPropertyValue('--cell-size'), 10) || 52;
    const wrap = document.getElementById('board-wrap');
    const availW = Math.max(wrap.clientWidth, 200) - 32;
    const fitW = Math.floor(availW / (cols + 2));
    return Math.min(base, Math.max(28, fitW));
}

export function renderBoard(state) {
    const { width, height, cells } = state.board;
    const cs = cellSize(width);
    const padding = cs;
    const boardW = (width + 1) * cs;
    const boardH = (height + 1) * cs;
    const label = Math.max(16, Math.round(cs * 0.42));
    const fontSize = Math.max(10, Math.round(cs * 0.26));

    const frame = document.getElementById('board-frame');
    frame.style.paddingTop = `${label}px`;
    frame.style.paddingLeft = `${label}px`;
    for (const el of frame.querySelectorAll('.board-coord')) el.remove();

    boardEl.innerHTML = '';
    boardEl.style.width = `${boardW}px`;
    boardEl.style.height = `${boardH}px`;

    boardEl.appendChild(buildSVG(width, height, cs, padding, boardW, boardH));

    for (let y = 0; y < height; y++) {
        for (let x = 0; x < width; x++) {
            const intn = document.createElement('div');
            intn.className = 'intersection';
            intn.dataset.x = x;
            intn.dataset.y = y;
            intn.style.left = `${padding + x * cs}px`;
            intn.style.top = `${padding + y * cs}px`;
            intn.style.width = `${cs}px`;
            intn.style.height = `${cs}px`;

            const pieceData = cells[y] && cells[y][x];
            if (pieceData) {
                intn.appendChild(createPieceElement(pieceData, false));
                intn.appendChild(createPieceTooltip(pieceData));
            }

            boardEl.appendChild(intn);
        }
    }

    for (let x = 0; x < width; x++) {
        const el = document.createElement('div');
        el.className = 'board-coord board-coord-top';
        el.style.width = `${cs}px`;
        el.style.height = `${label}px`;
        el.style.left = `${label + padding + x * cs - cs / 2}px`;
        el.style.top = `${label}px`;
        el.style.fontSize = `${fontSize}px`;
        el.textContent = numeral(x + 1);
        frame.appendChild(el);
    }

    for (let y = 0; y < height; y++) {
        const el = document.createElement('div');
        el.className = 'board-coord board-coord-left';
        el.style.width = `${label}px`;
        el.style.height = `${cs}px`;
        el.style.top = `${label + padding + y * cs - cs / 2}px`;
        el.style.fontSize = `${fontSize}px`;
        el.textContent = numeral(y + 1);
        frame.appendChild(el);
    }
}

function createPieceTooltip(piece) {
    const tooltip = document.createElement('div');
    tooltip.className = 'board-tooltip';
    tooltip.setAttribute('role', 'tooltip');

    const abilities = document.createElement('div');
    abilities.className = 'effective-abilities';

    const title = document.createElement('div');
    title.className = 'effective-abilities-title';
    title.textContent = '实时能力';
    abilities.appendChild(title);

    const list = document.createElement('div');
    list.className = 'effective-abilities-list';
    for (const ability of piece.effective_abilities || []) {
        const item = document.createElement('span');
        item.className = ability.effective ? 'effective' : 'ineffective';
        item.textContent = ability.name;
        list.appendChild(item);
    }
    abilities.appendChild(list);
    tooltip.appendChild(abilities);
    return tooltip;
}

function buildSVG(width, height, cs, padding, w, h) {
    const svg = document.createElementNS(SVG_NS, 'svg');
    svg.setAttribute('width', String(w));
    svg.setAttribute('height', String(h));
    svg.classList.add('board-lines');

    const x1 = padding;
    const y1 = padding;
    const x2 = padding + (width - 1) * cs;
    const y2 = padding + (height - 1) * cs;

    for (let x = 0; x < width; x++) {
        const cx = padding + x * cs;
        const line = document.createElementNS(SVG_NS, 'line');
        line.setAttribute('x1', String(cx));
        line.setAttribute('y1', String(y1));
        line.setAttribute('x2', String(cx));
        line.setAttribute('y2', String(y2));
        svg.appendChild(line);
    }

    for (let y = 0; y < height; y++) {
        const cy = padding + y * cs;
        const line = document.createElementNS(SVG_NS, 'line');
        line.setAttribute('x1', String(x1));
        line.setAttribute('y1', String(cy));
        line.setAttribute('x2', String(x2));
        line.setAttribute('y2', String(cy));
        svg.appendChild(line);
    }

    return svg;
}

export function getIntersection(x, y) {
    return boardEl.querySelector(`.intersection[data-x="${x}"][data-y="${y}"]`);
}

export function clearBoard() {
    boardEl.innerHTML = '';
}

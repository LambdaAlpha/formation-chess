import { createPieceElement } from './pieces.js';

const SVG_NS = 'http://www.w3.org/2000/svg';
const boardEl = document.getElementById('board');

function cellSize(cols) {
    const base = parseInt(getComputedStyle(document.documentElement).getPropertyValue('--cell-size'), 10) || 52;
    const wrap = document.getElementById('board-wrap');
    const availW = Math.max(wrap.clientWidth, 200) - 32;
    const fitW = Math.floor(availW / cols);
    return Math.min(base, Math.max(28, fitW));
}

export function renderBoard(state) {
    const { width, height, cells } = state.board;
    const cs = cellSize(width);
    const padding = cs / 2;
    const boardW = width * cs;
    const boardH = height * cs;

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
            }

            boardEl.appendChild(intn);
        }
    }
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

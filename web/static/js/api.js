const BASE = '';

export async function getState() {
    const r = await fetch(`${BASE}/api/state`);
    return r.json();
}

export async function postAction(action) {
    const r = await fetch(`${BASE}/api/action`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(action),
    });
    return r.json();
}

export async function postHints(request) {
    const r = await fetch(`${BASE}/api/hints`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(request),
    });
    return r.json();
}

export async function postNew(config) {
    const r = await fetch(`${BASE}/api/new`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(config),
    });
    if (!r.ok) {
        const err = await r.json();
        throw new Error(err.error || `HTTP ${r.status}`);
    }
    return r.json();
}

export async function getRules() {
    const r = await fetch(`${BASE}/api/rules`);
    return r.json();
}

export async function postUndo() {
    const r = await fetch(`${BASE}/api/undo`, { method: 'POST' });
    return r.json();
}

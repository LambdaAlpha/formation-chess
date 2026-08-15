const BASE = '';

async function requestJson(path, options = {}) {
    const response = await fetch(`${BASE}${path}`, options);
    const data = await response.json().catch(() => ({}));
    if (!response.ok) {
        throw new Error(data.error || `HTTP ${response.status}`);
    }
    return data;
}

function jsonOptions(body) {
    return {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
    };
}

export function getState() {
    return requestJson('/api/state');
}

export function postAction(request) {
    return requestJson('/api/action', jsonOptions(request));
}

export function postAgentAnalyze(request) {
    return requestJson('/api/agent/analyze', jsonOptions(request));
}

export function postAgentStep(request) {
    return requestJson('/api/agent/step', jsonOptions(request));
}

export function postNew(request) {
    return requestJson('/api/new', jsonOptions(request));
}

export function getRules() {
    return requestJson('/api/rules');
}

export function getAbilities() {
    return requestJson('/api/abilities');
}

export function postUndo(request) {
    return requestJson('/api/undo', jsonOptions(request));
}

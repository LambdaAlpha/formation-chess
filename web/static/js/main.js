import {
    init,
    render,
    setStatus,
    showAnalysis,
    clearAnalysis,
    setBusy,
    showPlayedAction,
} from './ui.js';
import {
    getState,
    postAction,
    postAgentAnalyze,
    postAgentStep,
    postNew,
    postUndo,
} from './api.js';

let currentState = null;
let requestInFlight = false;

async function main() {
    init(handleCommand);
    requestInFlight = true;
    setBusy(true);
    try {
        renderState(await getState());
    } catch (error) {
        setStatus('无法连接到服务器：' + error.message, true);
    } finally {
        requestInFlight = false;
        setBusy(false);
    }
}

function renderState(state) {
    currentState = state;
    render(state);
}

function versionedRequest() {
    return { revision: currentState.revision, side: currentState.player };
}

async function handleCommand(command) {
    if (requestInFlight || !currentState) return;
    requestInFlight = true;
    setBusy(true);

    try {
        if (command.type === 'new_game') {
            await createNewGame(command);
        } else if (command.type === 'undo') {
            await undo();
        } else if (command.type === 'agent_analyze') {
            await analyze();
        } else if (command.type === 'agent_step') {
            clearAnalysis();
            await runAgentStep(false);
        } else if (command.type === 'submit_action') {
            await submitAction(command.action);
        }
    } catch (error) {
        setStatus('网络错误：' + error.message, true);
    } finally {
        requestInFlight = false;
        setBusy(false);
    }
}

async function createNewGame(command) {
    clearAnalysis();
    const { type, ...config } = command;
    const state = await postNew({ ...versionedRequest(), ...config });
    renderState(state);
    setStatus(config.random_placement ? '随机布局已生成' : '新棋局已创建');
    await maybeRunAutomaticAgent();
}

async function undo() {
    clearAnalysis();
    const response = await postUndo(versionedRequest());
    renderState(response);
    if (response.error) {
        setStatus(response.error, true);
    } else {
        setStatus('已悔棋');
    }
}

async function analyze() {
    const response = await postAgentAnalyze({ ...versionedRequest(), top_k: 5 });
    if (!currentState || response.revision !== currentState.revision) return;
    showAnalysis(response);
    setStatus('已生成 ' + response.candidates.length + ' 个候选');
}

async function submitAction(action) {
    clearAnalysis();
    const response = await postAction({ ...versionedRequest(), action });
    renderState(response);
    if (response.error) {
        setStatus(response.error, true);
        return;
    }
    await maybeRunAutomaticAgent();
}

async function maybeRunAutomaticAgent() {
    if (!currentState || !currentState.can_agent_step) return;
    const bothAgents = currentState.controllers.red.control === 'agent'
        && currentState.controllers.black.control === 'agent';
    if (bothAgents) return;
    await runAgentStep(true);
}

async function runAgentStep(automatic) {
    const response = await postAgentStep(versionedRequest());
    renderState(response);
    if (response.error) {
        setStatus(response.error, true);
        return;
    }
    if (response.played) {
        showPlayedAction(response.played.action);
        const prefix = automatic ? 'AI 应手：' : 'AI 行棋：';
        setStatus(prefix + response.played.notation);
    }
}

main();

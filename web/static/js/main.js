import { init, render, setStatus, hidePopup } from './ui.js';
import { getState, postAction, postNew, postUndo } from './api.js';
import { cachePieceListFromState } from './pieces.js';

async function main() {
    init(handleAction);

    try {
        const state = await postNew({ board: { width: 9, height: 10 } });
        cachePieceListFromState(state);
        render(state);
    } catch (e) {
        setStatus('无法连接到服务器', true);
    }
}

async function handleAction(actionReq) {
    hidePopup();

    if (actionReq.type === 'new_game') {
        const { type, ...config } = actionReq;
        try {
            const state = await postNew(config);
            cachePieceListFromState(state);
            render(state);
        } catch (e) {
            setStatus(e.message, true);
        }
        return;
    }

    if (actionReq.type === 'undo') {
        try {
            const response = await postUndo();
            if (response.error) {
                setStatus(response.error, true);
            } else {
                setStatus('已悔棋');
            }
            render(response);
        } catch (e) {
            setStatus('网络错误: ' + e.message, true);
        }
        return;
    }

    try {
        const response = await postAction(actionReq);
        if (response.error) {
            setStatus(response.error, true);
        }
        render(response);
    } catch (e) {
        setStatus('网络错误: ' + e.message, true);
    }
}

main();

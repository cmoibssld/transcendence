const ws = new WebSocket('ws://127.0.0.1:3000/ws');
const LOBBY_URL = '/';

const statusEl = document.getElementById('status');
const tableIdEl = document.getElementById('tableId');
const currentPlayerEl = document.getElementById('currentPlayer');
const dealerCardsEl = document.getElementById('dealerCards');
const playerCardsEl = document.getElementById('playerCards');
const logListEl = document.getElementById('logList');
const joinButton = document.getElementById('joinButton');
const hitButton = document.getElementById('hitButton');
const stayButton = document.getElementById('stayButton');
const leaveButton = document.getElementById('leaveButton');

let pendingJoin = false;
let pendingLeave = false;
let clientId = null;
let outcomeLocked = false;
let pauseTimeoutId = null;
let pendingInfoDuringPause = null;

const AUTO_PAUSE_MS = 2500;

function setStatus(message) {
    statusEl.textContent = message;
}

function showError(message) {
    setStatus(`Erreur: ${message}`);
    appendLog(message);
}

function showLobbyControls() {
    joinButton.hidden = false;
    joinButton.style.display = 'inline-block';
    joinButton.disabled = false;

    hitButton.hidden = true;
    hitButton.style.display = 'none';
    hitButton.disabled = true;

    stayButton.hidden = true;
    stayButton.style.display = 'none';
    stayButton.disabled = true;

    leaveButton.hidden = true;
    leaveButton.style.display = 'none';
    leaveButton.disabled = true;
}

function showGameControls() {
    joinButton.hidden = true;
    joinButton.style.display = 'none';
    joinButton.disabled = true;

    hitButton.hidden = false;
    hitButton.style.display = 'inline-block';
    hitButton.disabled = false;

    stayButton.hidden = false;
    stayButton.style.display = 'inline-block';
    stayButton.disabled = false;

    leaveButton.hidden = false;
    leaveButton.style.display = 'inline-block';
    leaveButton.disabled = false;
}

showLobbyControls();

ws.onmessage = (event) => {
    let data;
    try {
        data = JSON.parse(event.data);
    } catch (error) {
        console.error('Invalid JSON from server', error, event.data);
        showError('Réponse serveur invalide.');
        return;
    }
    console.log(data);

    if (data.error) {
        showError(data.error);
    }

    if (data.status === 'joined') {
        if (!pendingJoin) {
            return;
        }

        pendingJoin = false;
        pendingLeave = false;
        clientId = data.player_id || clientId;
        showGameControls();
        setStatus(`Partie rejointe. Player ID: ${clientId ?? 'inconnu'}`);
        return;
    }

    if (data.status === 'waiting') {
        showLobbyControls();

        if (pendingLeave) {
            pendingLeave = false;
            pendingJoin = false;
            setStatus('Retour au lobby...');
            window.location.href = LOBBY_URL;
            return;
        }

        if (pendingJoin) {
            pendingJoin = false;
            setStatus('Join refusé pour l’instant (table pleine / en attente).');
            return;
        }

        setStatus('En attente dans le lobby.');
        return;
    }

    if (data.table_id !== undefined || data.current_player !== undefined) {
        if (outcomeLocked) {
            pendingInfoDuringPause = data;
            return;
        }
        renderGameInfo(data);
        if (pendingJoin) {
            pendingJoin = false;
            showGameControls();
            setStatus('Partie rejointe.');
        }
    }
};

ws.onopen = () => {
    setStatus('Connecté au serveur. En attente du join...');
};

ws.onclose = () => {
    setStatus('Connexion WebSocket fermée.');
};

ws.onerror = () => {
    showError('WebSocket.');
};

function hit() {
    if (outcomeLocked) {
        return;
    }
    ws.send(JSON.stringify({ action: 'hit' }));
}

function stay() {
    if (outcomeLocked) {
        return;
    }
    ws.send(JSON.stringify({ action: 'stay' }));
}

function join() {
	pendingJoin = true;
	pendingLeave = false;
	ws.send(JSON.stringify({ action: 'join' }));
    setStatus('Demande de join envoyée...');
}

function leave() {
	pendingLeave = true;
	pendingJoin = false;
	ws.send(JSON.stringify({ action: 'leave' }));
    setStatus('Demande de leave envoyée...');
}

function renderGameInfo(info) {
    let dealerCardsCount = 0;
    let dealerPoints;
    if (info.table_id !== undefined) {
        tableIdEl.textContent = `Table: ${info.table_id}`;
    }

    if (info.current_player !== undefined) {
        currentPlayerEl.textContent = `Tour du joueur: ${info.current_player}`;
    }

    if (Array.isArray(info.dealer_cards)) {
        dealerCardsCount = info.dealer_cards.length;
        dealerPoints = info.dealer_points;
        renderCards(dealerCardsEl, info.dealer_cards, dealerPoints);
    }

    if (info.players_cards && typeof info.players_cards === 'object') {
        const playerIds = info.players_in_game || Object.keys(info.players_cards);
        const targetId = clientId && info.players_cards[clientId] ? clientId : playerIds[0];
        const cards = targetId ? info.players_cards[targetId] : [];
        const points = info.players_points && targetId ? info.players_points[targetId] : undefined;

        if (Array.isArray(cards)) {
            renderCards(playerCardsEl, cards, points, targetId);

            if (typeof points === 'number') {
                const outcome = evaluateOutcome(points, dealerPoints, dealerCardsCount);
                if (outcome) {
                    handleRoundEnd(outcome);
                }
            }
        }
    }
}

function renderCards(container, cards, points, playerId) {
    container.innerHTML = '';
    const labels = cards.length > 0 ? cards.map(formatCard) : ['—'];

    labels.forEach((cardLabel) => {
        const slot = document.createElement('div');
        slot.className = 'card-slot' + (cardLabel !== '—' ? ' filled' : '');
        slot.textContent = cardLabel;
        container.appendChild(slot);
    });

    if (typeof points === 'number') {
        const pointsSlot = document.createElement('div');
        pointsSlot.className = 'card-slot filled';
        pointsSlot.textContent = `Points: ${points}`;
        container.appendChild(pointsSlot);
    }

    if (playerId) {
        container.setAttribute('aria-label', `Joueur ${playerId}`);
    }
}

function formatCard(card) {
    if (card === null || card === undefined) {
        return '—';
    }
    if (typeof card === 'string' || typeof card === 'number') {
        return String(card);
    }
    if (typeof card === 'object') {
        const rawValue = card.value ?? card.rank ?? card.number ?? card.v;
        const rawColor = card.color ?? card.suit ?? card.c;

        const valueLabel = formatCardValue(rawValue);
        const suitLabel = formatCardSuit(rawColor);

        if (valueLabel || suitLabel) {
            return `${valueLabel}${suitLabel}`.trim();
        }
        try {
            return JSON.stringify(card);
        } catch {
            return '[Objet carte]';
        }
    }
    return String(card);
}

function formatCardValue(value) {
    if (value === undefined || value === null) {
        return '';
    }
    const numeric = Number(value);
    if (Number.isNaN(numeric)) {
        return String(value);
    }
    switch (numeric) {
        case 1:
            return 'A';
        case 11:
            return 'V';
        case 12:
            return 'D';
        case 13:
            return 'R';
        default:
            return String(numeric);
    }
}

function formatCardSuit(color) {
    if (color === undefined || color === null) {
        return '';
    }
    const numeric = Number(color);
    if (Number.isNaN(numeric)) {
        return String(color);
    }
    switch (numeric) {
        case 1:
            return '♥';
        case 2:
            return '♠';
        case 3:
            return '♦';
        case 4:
            return '♣';
        default:
            return '';
    }
}

function appendLog(message) {
    if (!logListEl) {
        return;
    }
    const item = document.createElement('li');
    const time = new Date().toLocaleTimeString('fr-FR');
    item.textContent = `[${time}] ${message}`;
    logListEl.prepend(item);

    if (logListEl.children.length > 8) {
        logListEl.removeChild(logListEl.lastElementChild);
    }
}

function evaluateOutcome(playerPoints, dealerPoints, dealerCardsCount) {
    if (playerPoints > 21) {
        return 'Vous avez dépassé 21: perdu.';
    }

    if (dealerCardsCount <= 1 || typeof dealerPoints !== 'number') {
        return null;
    }

    if (dealerPoints > 21) {
        return 'Le croupier a dépassé 21: gagné.';
    }

    if (playerPoints > dealerPoints) {
        return 'Vous gagnez.';
    }

    if (playerPoints < dealerPoints) {
        return 'Vous perdez.';
    }

    return 'Égalité.';
}

function handleRoundEnd(message) {
    if (outcomeLocked) {
        return;
    }
    outcomeLocked = true;
    setStatus(message);
    setControlsPaused(true);

    if (pauseTimeoutId) {
        clearTimeout(pauseTimeoutId);
    }
    pauseTimeoutId = setTimeout(() => {
        setControlsPaused(false);
        outcomeLocked = false;
        if (pendingInfoDuringPause) {
            renderGameInfo(pendingInfoDuringPause);
            pendingInfoDuringPause = null;
        }
    }, AUTO_PAUSE_MS);
}

function setControlsPaused(paused) {
    hitButton.disabled = paused;
    stayButton.disabled = paused;
}

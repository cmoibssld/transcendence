const urlParams = new URLSearchParams(window.location.search);
const storedTableId = window.localStorage.getItem('blackjack-table-id');
const tableId = urlParams.get('table_id') || storedTableId || '1';
window.localStorage.setItem('blackjack-table-id', tableId);

const ws = new WebSocket(`ws://${window.location.host}/ws/?table_id=${encodeURIComponent(tableId)}`);
const LOBBY_URL = '/';

const statusEl = document.getElementById('status');
const tableIdEl = document.getElementById('tableId');
const currentPlayerEl = document.getElementById('currentPlayer');
const dealerCardsEl = document.getElementById('dealerCards');
const playerCardsEl = document.getElementById('playerCards');
const joinButton = document.getElementById('joinButton');
const hitButton = document.getElementById('hitButton');
const stayButton = document.getElementById('stayButton');
const leaveButton = document.getElementById('leaveButton');

let pendingJoin = false;
let pendingLeave = false;
let clientId = null;

function setStatus(message) {
    statusEl.textContent = message;
}

function showError(message) {
    setStatus(`Erreur: ${message}`);
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
    ws.send(JSON.stringify({ action: 'hit' }));
}

function stay() {
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
        const cards = info.dealer_cards.join(', ');
        const points = dealerPoints !== undefined ? ` (${dealerPoints})` : '';
        dealerCardsEl.textContent = `${cards}${points}`;
    }

    if (info.players_cards && typeof info.players_cards === 'object') {
        const playerIds = info.players_in_game || Object.keys(info.players_cards);
        const targetId = clientId && info.players_cards[clientId] ? clientId : playerIds[0];
        const cards = targetId ? info.players_cards[targetId] : [];
        const points = info.players_points && targetId ? info.players_points[targetId] : undefined;

        if (Array.isArray(cards)) {
            const pointsLabel = points !== undefined ? ` (${points})` : '';
            const playerLabel = targetId ? `Joueur ${targetId}: ` : '';
            playerCardsEl.textContent = `${playerLabel}${cards.join(', ')}${pointsLabel}`;

            if (typeof points === 'number') {
                updateLocalOutcome(points, dealerPoints, dealerCardsCount);
            }
        }
    }
}

function updateLocalOutcome(playerPoints, dealerPoints, dealerCardsCount) {
    if (playerPoints > 21) {
        setStatus('Vous avez dépassé 21: perdu.');
        return;
    }

    if (dealerCardsCount <= 1 || typeof dealerPoints !== 'number') {
        return;
    }

    if (dealerPoints > 21) {
        setStatus('Le croupier a dépassé 21: gagné.');
        return;
    }

    if (playerPoints > dealerPoints) {
        setStatus('Vous gagnez.');
        return;
    }

    if (playerPoints < dealerPoints) {
        setStatus('Vous perdez.');
        return;
    }

    setStatus('Égalité.');
}

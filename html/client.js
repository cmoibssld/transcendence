const ws = new WebSocket('ws://127.0.0.1:8080/ws/');
const LOBBY_URL = '/';

const statusEl = document.getElementById('status');
const joinButton = document.getElementById('joinButton');
const hitButton = document.getElementById('hitButton');
const stayButton = document.getElementById('stayButton');
const leaveButton = document.getElementById('leaveButton');

let pendingJoin = false;
let pendingLeave = false;

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
    const data = JSON.parse(event.data);
    console.log(data);

    if (!data.status) {
        return;
    }

    if (data.status === 'joined') {
        if (!pendingJoin) {
            return;
        }

        pendingJoin = false;
        pendingLeave = false;
        showGameControls();
        statusEl.textContent = `Partie rejointe. Player ID: ${data.player_id}`;
        return;
    }

    if (data.status === 'waiting') {
        showLobbyControls();

        if (pendingLeave) {
            pendingLeave = false;
            pendingJoin = false;
            statusEl.textContent = 'Retour au lobby...';
            window.location.href = LOBBY_URL;
            return;
        }

        if (pendingJoin) {
            pendingJoin = false;
            statusEl.textContent = 'Join refusé pour l’instant (table pleine / en attente).';
            return;
        }

        statusEl.textContent = 'En attente dans le lobby.';
    }
};

ws.onopen = () => {
    statusEl.textContent = 'Connecté au serveur. En attente du join...';
};

ws.onclose = () => {
    statusEl.textContent = 'Connexion WebSocket fermée.';
};

ws.onerror = () => {
    statusEl.textContent = 'Erreur WebSocket.';
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
    statusEl.textContent = 'Demande de join envoyée...';
}

function leave() {
	pendingLeave = true;
	pendingJoin = false;
	ws.send(JSON.stringify({ action: 'leave' }));
    statusEl.textContent = 'Demande de leave envoyée...';
}

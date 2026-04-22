const ws = new WebSocket('ws://127.0.0.1:8080/ws/');
const LOBBY_URL = '/';

const statusEl = document.getElementById('status');
const joinButton = document.getElementById('joinButton');
const hitButton = document.getElementById('hitButton');
const stayButton = document.getElementById('stayButton');
const leaveButton = document.getElementById('leaveButton');

function showLobbyControls() {
    joinButton.hidden = false;
    hitButton.hidden = true;
    stayButton.hidden = true;
    leaveButton.hidden = true;
}

function showGameControls() {
    joinButton.hidden = true;
    hitButton.hidden = false;
    stayButton.hidden = false;
    leaveButton.hidden = false;
}

showLobbyControls();

ws.onmessage = (event) => {
    const gameState = JSON.parse(event.data);
    console.log(gameState);
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
	ws.send(JSON.stringify({ action: 'join' }));
    showGameControls();
    statusEl.textContent = 'Partie rejointe.';
}

function leave() {
	ws.send(JSON.stringify({ action: 'leave' }));
    showLobbyControls();
    statusEl.textContent = 'Retour au lobby...';
    window.location.href = LOBBY_URL;
}

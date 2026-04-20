const ws = new WebSocket('ws://127.0.0.1:8080/ws/');

ws.onmessage = (event) => {
    const gameState = JSON.parse(event.data);
    console.log(gameState);
};

function hit() {
    ws.send(JSON.stringify({ action: 'hit' }));
}

function stay() {
    ws.send(JSON.stringify({ action: 'stay' }));
}

function join() {
	ws.send(JSON.stringify({ action: 'join' }));
}

function leave() {
	ws.send(JSON.stringify({ action: 'leave' }));
}

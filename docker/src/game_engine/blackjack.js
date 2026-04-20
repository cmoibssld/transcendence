const fs = require('fs');

const IO_FILE = process.env.GAME_IO_FILE || '/app/game.json';
const TICK_MS = Number(process.env.GAME_ENGINE_TICK_MS || 200);

class Dealer {
    constructor() {
        this.sum = 0;
        this.hiddenSum = 0;
        this.aceCount = 0;
        this.cards = [];
        this.hidden = null;
    }

    reduceAce() {
        while (this.sum > 21 && this.aceCount > 0) {
            this.sum -= 10;
            this.aceCount -= 1;
        }
    }
}

class Player {
    constructor(id) {
        this.id = id;
        this.sum = 0;
        this.aceCount = 0;
        this.canHit = true;
        this.cards = [];
        this.blackjack = false;
        this.result = null;
    }

    reduceAce() {
        while (this.sum > 21 && this.aceCount > 0) {
            this.sum -= 10;
            this.aceCount -= 1;
        }
    }
}

class Deck {
    constructor() {
        this.cards = buildDeck();
    }

    shuffleDeck() {
        for (let i = this.cards.length - 1; i > 0; i--) {
            const j = Math.floor(Math.random() * (i + 1));
            const tmp = this.cards[i];
            this.cards[i] = this.cards[j];
            this.cards[j] = tmp;
        }
    }

    draw() {
        return this.cards.pop();
    }
}

class Table {
    constructor(id, playerIds = []) {
        this.id = id;
        this.dealer = new Dealer();
        this.players = playerIds.map((playerId) => new Player(playerId));
        this.deck = new Deck();
        this.phase = 'waiting'; // waiting | playing | finished
        this.currentPlayerIndex = 0;
    }

    initGame() {
        if (this.players.length === 0) {
            throw new Error('No players for this table');
        }

        this.deck.shuffleDeck();

        const hidden = this.deck.draw();
        this.dealer.hidden = hidden;
        this.dealer.hiddenSum = getValue(hidden);
        this.dealer.aceCount += checkAce(hidden);

        const visible = this.deck.draw();
        this.dealer.cards.push(visible);
        this.dealer.sum += getValue(visible);
        this.dealer.aceCount += checkAce(visible);
        this.dealer.reduceAce();

        for (const player of this.players) {
            for (let i = 0; i < 2; i++) {
                const card = this.deck.draw();
                player.cards.push(card);
                player.sum += getValue(card);
                player.aceCount += checkAce(card);
                player.reduceAce();
            }

            if (player.sum === 21) {
                player.blackjack = true;
                player.canHit = false;
            }
        }

        this.phase = 'playing';
        this.advanceTurnIfNeeded();
    }

    getCurrentPlayer() {
        if (this.phase !== 'playing') {
            return null;
        }
        return this.players[this.currentPlayerIndex] || null;
    }

    hit(player) {
        if (!player.canHit) {
            return;
        }

        const card = this.deck.draw();
        if (!card) {
            throw new Error('Deck is empty');
        }

        player.cards.push(card);
        player.sum += getValue(card);
        player.aceCount += checkAce(card);
        player.reduceAce();

        if (player.sum >= 21) {
            player.canHit = false;
        }
    }

    applyAction(playerId, action) {
        if (this.phase !== 'playing') {
            throw new Error('Game is not in playing phase');
        }

        const currentPlayer = this.getCurrentPlayer();
        if (!currentPlayer) {
            throw new Error('No current player');
        }

        if (currentPlayer.id !== playerId) {
            throw new Error('Not your turn');
        }

        if (action === 'hit') {
            this.hit(currentPlayer);
        } else if (action === 'stay') {
            currentPlayer.canHit = false;
        } else {
            throw new Error('Invalid action');
        }

        this.advanceTurnIfNeeded();
    }

    advanceTurnIfNeeded() {
        while (
            this.currentPlayerIndex < this.players.length
            && this.players[this.currentPlayerIndex].canHit === false
        ) {
            this.currentPlayerIndex += 1;
        }

        if (this.currentPlayerIndex >= this.players.length) {
            this.endGame();
        }
    }

    endGame() {
        this.dealer.cards.push(this.dealer.hidden);
        this.dealer.sum += this.dealer.hiddenSum;
        this.dealer.reduceAce();

        while (this.dealer.sum < 17) {
            const card = this.deck.draw();
            if (!card) {
                break;
            }

            this.dealer.cards.push(card);
            this.dealer.sum += getValue(card);
            this.dealer.aceCount += checkAce(card);
            this.dealer.reduceAce();
        }

        for (const player of this.players) {
            if (player.sum > 21) {
                player.result = 'lose';
            } else if (this.dealer.sum > 21) {
                player.result = 'win';
            } else if (player.sum === this.dealer.sum) {
                player.result = 'push';
            } else if (player.sum > this.dealer.sum) {
                player.result = 'win';
            } else {
                player.result = 'lose';
            }
        }

        this.phase = 'finished';
    }

    publicState() {
        const isPlaying = this.phase === 'playing';

        return {
            state: 'state',
            tableId: this.id,
            phase: this.phase,
            remainingCards: this.deck.cards.length,
            currentPlayerId: isPlaying ? (this.getCurrentPlayer() ? this.getCurrentPlayer().id : null) : null,
            dealer: {
                cards: isPlaying ? [...this.dealer.cards, 'XX'] : [...this.dealer.cards],
                sum: this.dealer.sum,
            },
            players: this.players.map((player) => ({
                id: player.id,
                cards: [...player.cards],
                sum: player.sum,
                canHit: player.canHit,
                blackjack: player.blackjack,
                result: player.result,
            })),
        };
    }
}

class GameEngine {
    constructor() {
        this.tables = new Map();
    }

    handle(request) {
        switch (request.state) {
        case 'init_table': {
            const tableId = request.tableId;
            const players = request.players;

            if (!tableId || !Array.isArray(players) || players.length === 0) {
                throw new Error('init_table requires: tableId and players[]');
            }

            const table = new Table(tableId, players);
            table.initGame();
            this.tables.set(tableId, table);

            return {
                ok: true,
                state: table.publicState(),
            };
        }
        case 'action': {
            const tableId = request.tableId;
            const playerId = request.playerId;
            const action = request.action;

            const table = this.tables.get(tableId);
            if (!table) {
                throw new Error('Unknown tableId');
            }

            table.applyAction(playerId, action);
            return {
                ok: true,
                state: table.publicState(),
            };
        }
        case 'get_state': {
            const tableId = request.tableId;
            const table = this.tables.get(tableId);

            if (!table) {
                throw new Error('Unknown tableId');
            }

            return {
                ok: true,
                state: table.publicState(),
            };
        }
        default:
            throw new Error('Unknown request state');
        }
    }
}

////////////////////UTILS///////////////////////////////

function getValue(card) {
    const data = card.split('-');
    const value = data[0];

    if (Number.isNaN(Number(value))) {
        if (value === 'A') {
            return 11;
        }
        return 10;
    }

    return parseInt(value, 10);
}

function buildDeck() {
    const values = ['A', '2', '3', '4', '5', '6', '7', '8', '9', '10', 'J', 'Q', 'K'];
    const types = ['C', 'D', 'H', 'S'];
    const deck = [];

    for (let i = 0; i < types.length; i++) {
        for (let j = 0; j < values.length; j++) {
            deck.push(values[j] + '-' + types[i]);
        }
    }
    return deck;
}

function checkAce(card) {
    if (card[0] === 'A') {
        return 1;
    }
    return 0;
}

////////////////////FILE I/O LOOP///////////////////////////////

function ensureIoFile() {
    if (!fs.existsSync(IO_FILE)) {
        fs.writeFileSync(
            IO_FILE,
            JSON.stringify({ request: null, response: null, lastProcessedRequestId: null }, null, 2),
            'utf8',
        );
    }
}

function readJson(path) {
    const raw = fs.readFileSync(path, 'utf8');
    if (!raw || !raw.trim()) {
        return {};
    }
    return JSON.parse(raw);
}

function writeJsonAtomic(path, data) {
    const tmpFile = `${path}.tmp`;
    fs.writeFileSync(tmpFile, JSON.stringify(data, null, 2), 'utf8');
    fs.renameSync(tmpFile, path);
}

function normalizeRequestEnvelope(data) {
    if (!data) {
        return { request: null, response: null, lastProcessedRequestId: null };
    }

    if (data.request) {
        return {
            request: data.request,
            response: data.response || null,
            lastProcessedRequestId: data.lastProcessedRequestId || null,
        };
    }

    if (typeof data.state === 'string' && data.state !== 'state') {
        return {
            request: data,
            response: null,
            lastProcessedRequestId: null,
        };
    }

    return {
        request: null,
        response: data.response || null,
        lastProcessedRequestId: data.lastProcessedRequestId || null,
    };
}

const engine = new GameEngine();
let lastSeenRequestId = null;

function processOnce() {
    let rawData;
    try {
        rawData = readJson(IO_FILE);
    } catch (error) {
        return;
    }

    const data = normalizeRequestEnvelope(rawData);
    const request = data.request;

    if (!request) {
        return;
    }

    if (!request.id) {
        const fallbackId = Date.now().toString();
        request.id = fallbackId;
    }

    if (request.id === lastSeenRequestId || request.id === data.lastProcessedRequestId) {
        return;
    }

    try {
        const result = engine.handle(request);
        data.response = {
            requestId: request.id,
            ok: true,
            ...result,
            at: new Date().toISOString(),
        };
    } catch (error) {
        data.response = {
            requestId: request.id,
            ok: false,
            error: error.message,
            at: new Date().toISOString(),
        };
    }

    data.lastProcessedRequestId = request.id;
    data.request = null;
    lastSeenRequestId = request.id;

    writeJsonAtomic(IO_FILE, data);
    console.log(`[engine] processed request ${request.id}`);
}

ensureIoFile();
console.log(`[engine] started, watching ${IO_FILE}`);
setInterval(processOnce, TICK_MS);

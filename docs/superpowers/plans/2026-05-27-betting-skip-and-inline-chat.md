# Betting Skip + Integrated Table Chat Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When the betting countdown expires, the player sits out the round as a passive spectator instead of receiving an automatic bet; and a per-table shared chat panel is embedded in the game board as a third column, reusing the existing game WebSocket.

**Architecture:** Two features sharing one plan because both touch `GameBoard.jsx`. The skip feature adds a `game.skip` action in the Rust game-engine that removes the player from the round without a bet, then rewires the frontend timer. The chat feature multiplexes chat messages over the existing game WebSocket (the `ws_service` already routes `chat.*` to the chat service and `game.*` to the game-engine), and adds an `InlineChat` sub-component in a new third grid column.

**Tech Stack:** Rust/actix-web (game-engine), React 19/Vite (front-end), WebSocket (ws_service routing already in place)

---

### Task 1: Backend — `skip_player` method in GameInfo

**Files:**
- Modify: `game-engine/src/blackjack.rs`

- [ ] **Step 1: Add `skip_player` to `GameInfo` after the `place_bet` method (after line 138)**

```rust
/// Remove a player from this round without placing a bet.
/// Returns:
///   None    – player not in pending_bets or phase is not Betting (invalid call)
///   Some(0) – skipped; still waiting for other players to bet/skip
///   Some(1) – pending_bets now empty and players remain → start playing
///   Some(2) – pending_bets now empty and no players remain → restart
pub fn skip_player(&mut self, player_id: u32) -> Option<u8> {
    if self.phase != Phase::Betting {
        return None;
    }
    let idx = self.pending_bets.iter().position(|&p| p == player_id)?;
    self.pending_bets.remove(idx);
    self.players_in_game.retain(|&p| p != player_id);
    self.players_cards.remove(&player_id);
    self.players_points.remove(&player_id);

    if !self.pending_bets.is_empty() {
        return Some(0); // still waiting for others
    }
    if self.players_in_game.is_empty() {
        Some(2) // everyone skipped — restart needed
    } else {
        Some(1) // all remaining players are ready — deal and play
    }
}
```

- [ ] **Step 2: Build to verify it compiles**

```bash
cd /home/elark/transcendance/game-engine && cargo build 2>&1
```

Expected: clean build (no errors).

- [ ] **Step 3: Commit**

```bash
cd /home/elark/transcendance
git add game-engine/src/blackjack.rs
git commit -m "feat(game): add skip_player to GameInfo — sit-out without auto-bet"
```

---

### Task 2: Backend — `do_skip` handler in GameData

**Files:**
- Modify: `game-engine/src/manager.rs`

- [ ] **Step 1: Add `"game.skip"` arm to the `handle_ingest` match (around line 213)**

Replace the entire `handle_ingest` method body:

```rust
pub fn handle_ingest(&mut self, user_id: u64, kind: &str, payload: &serde_json::Value) -> IngestResult {
    match kind {
        "game.join"  => self.do_join(user_id, payload),
        "game.bet"   => self.do_bet(user_id, payload),
        "game.hit"   => self.do_hit(user_id),
        "game.stay"  => self.do_stay(user_id),
        "game.skip"  => self.do_skip(user_id),
        "game.leave" => self.do_leave(user_id),
        _ => IngestResult {
            personal: Some(serde_json::json!({
                "kind": "game.error", "payload": { "message": "unknown action" }
            })),
            broadcast: None, post_broadcast: None, audience: vec![], history: None, coin_updates: vec![],
        },
    }
}
```

- [ ] **Step 2: Add `do_skip` to `GameData` (add after `do_bet`, around line 324)**

```rust
fn do_skip(&mut self, user_id: u64) -> IngestResult {
    let (table_id, player_id) = match self.user_sessions.get(&user_id) {
        Some(&s) => s,
        None => return IngestResult {
            personal: Some(serde_json::json!({
                "kind": "game.error", "payload": { "message": "Not in a game" }
            })),
            broadcast: None, post_broadcast: None, audience: vec![user_id], history: None, coin_updates: vec![],
        },
    };

    let table = match self.tables.get_mut(&table_id) {
        Some(t) => t,
        None => return IngestResult {
            personal: None, broadcast: None, post_broadcast: None,
            audience: vec![], history: None, coin_updates: vec![],
        },
    };

    match table.info.skip_player(player_id) {
        None => IngestResult {
            personal: Some(serde_json::json!({
                "kind": "game.error", "payload": { "message": "Cannot skip now" }
            })),
            broadcast: None, post_broadcast: None, audience: vec![user_id], history: None, coin_updates: vec![],
        },
        Some(0) => {
            // Still waiting for other players
            let audience  = table.audience();
            let state_val = serde_json::to_value(&table.info).unwrap_or_default();
            IngestResult {
                personal: None,
                broadcast: Some(serde_json::json!({ "kind": "game.state", "payload": state_val })),
                post_broadcast: None, audience, history: None, coin_updates: vec![],
            }
        }
        Some(1) => {
            // All remaining players have bet — deal and start playing
            // deal_to_player silently skips players removed from players_cards (the one who skipped)
            table.info.start_playing();
            let ids: Vec<u32> = table.players.keys().cloned().collect();
            for pid in ids {
                table.info.deal_to_player(pid, &mut table.deck, 2);
            }
            let audience  = table.audience();
            let state_val = serde_json::to_value(&table.info).unwrap_or_default();
            IngestResult {
                personal: None,
                broadcast: Some(serde_json::json!({ "kind": "game.state", "payload": state_val })),
                post_broadcast: None, audience, history: None, coin_updates: vec![],
            }
        }
        Some(_) => {
            // Some(2): everyone skipped — restart the round immediately
            table.restart();
            let audience  = table.audience();
            let state_val = serde_json::to_value(&table.info).unwrap_or_default();
            IngestResult {
                personal: None,
                broadcast: Some(serde_json::json!({ "kind": "game.state", "payload": state_val })),
                post_broadcast: None, audience, history: None, coin_updates: vec![],
            }
        }
    }
}
```

- [ ] **Step 3: Build to verify it compiles**

```bash
cd /home/elark/transcendance/game-engine && cargo build 2>&1
```

Expected: clean build.

- [ ] **Step 4: Commit**

```bash
cd /home/elark/transcendance
git add game-engine/src/manager.rs
git commit -m "feat(game): do_skip handler — player sits out round on timer expiry"
```

---

### Task 3: Frontend — skip on timer expiry + spectator state

**Files:**
- Modify: `front-end/src/components/GameBoard.jsx`

- [ ] **Step 1: Add `skippedThisRound` state and a ref for async-closure access**

After the existing timer states (after line ~257 where `turnTimer` is declared):

```js
const [skippedThisRound, setSkippedThisRound] = useState(false)
const skippedRef            = useRef(false)
```

`skippedRef` mirrors the state and is safe to read inside WS event closures that would otherwise capture a stale value.

- [ ] **Step 2: Reset skip state on WS reconnect**

In the WS setup effect, in the block that resets all state (around line 305), add the two resets:

```js
setJoined(false); setGameInfo(null); setSelfId(null)
selfIdRef.current = null; setRoundEnd(null); setRevealSeq(null)
skippedRef.current = false; setSkippedThisRound(false)
clearRevealTimers()
setStatus(`Connexion à la table ${tableId}…`)
```

- [ ] **Step 3: Reset skip state in the `game.state` handler when a new round starts**

Replace the entire `game.state` handler block (starting at `if (kind === "game.state")`):

```js
if (kind === "game.state") {
    setGameInfo(payload)
    setJoined(true)
    // New round: server re-adds us to pending_bets → clear skip flag
    const selfNum = selfIdRef.current !== null ? Number(selfIdRef.current) : null
    if (selfNum !== null && (payload.pending_bets ?? []).some(p => Number(p) === selfNum)) {
        skippedRef.current = false
        setSkippedThisRound(false)
    }
    if (payload.phase === "Playing") {
        setStatus(`Table ${payload.table_id} — en cours`)
    } else {
        setStatus(`Table ${payload.table_id} — phase de mise`)
    }
    fetch(`${GAME_URL}/tables`).then(r => r.ok ? r.json() : []).then(setTables).catch(() => {})
    return
}
```

- [ ] **Step 4: Suppress OutcomeScreen for skipped spectators**

In the `game.round_end` handler, inside the final `setTimeout` (around line 402), replace the `setRoundEnd({...})` call with a guard:

```js
revealTimers.current.push(setTimeout(() => {
    setRevealSeq(null)
    if (!skippedRef.current) {
        setRoundEnd({
            dealerPoints:    dealerPts,
            dealerBlackjack: result.dealer_blackjack ?? false,
            allWinners:      [...Array.from(bjWinnersSet), ...Array.from(winnersSet)],
            losers:          Array.from(losersSet),
            outcomeClass,
            isBlackjack,
            isPush,
            myPoints,
            coinDelta,
        })
    }
    setStatus("Fin de manche")
    fetchCoins()
    fetch(`${GAME_URL}/tables`).then(r => r.ok ? r.json() : []).then(setTables).catch(() => {})
}, t + AFTER))
```

- [ ] **Step 5: Rewrite the betting timer effect to send `game.skip` on expiry**

Replace the betting timer `useEffect` (lines 488–504) entirely:

```js
useEffect(() => {
    if (!joined || phase !== "Betting" || !selfPendingBet) { setBettingTimer(null); return }
    setBettingTimer(BET_TIMER_MAX)
    let t = BET_TIMER_MAX
    const id = setInterval(() => {
        t -= 1
        setBettingTimer(t)
        if (t <= 0) {
            clearInterval(id)
            setBettingTimer(null)
            const ws = wsRef.current
            if (ws?.readyState === WebSocket.OPEN) {
                ws.send(JSON.stringify({ kind: "game.skip", payload: {} }))
            }
            skippedRef.current = true
            setSkippedThisRound(true)
        }
    }, 1000)
    return () => clearInterval(id)
}, [joined, phase, selfPendingBet])
```

- [ ] **Step 6: Show skip banner when the player has sat out**

Replace the actions/betting section at the bottom of the JSX (around line 675):

```jsx
{joined && phase === "Betting" ? (
    skippedThisRound ? (
        <div className="bet-panel bet-panel--waiting">
            <span className="bet-panel__msg">Vous avez passé cette manche — vous regardez</span>
        </div>
    ) : (
        <BettingPanel
            coins={coins ?? 0}
            selfPendingBet={selfPendingBet}
            onBet={handleBet}
            timer={bettingTimer}
        />
    )
) : (
    <div className="gb-actions">
        {isMyTurn && turnTimer !== null && (
            <TimerRing seconds={turnTimer} max={TURN_TIMER_MAX} urgent={10} />
        )}
        <button
            type="button"
            className="gb-btn gb-btn--hit"
            onClick={() => send("game.hit")}
            disabled={!joined || !isMyTurn}
        >Hit</button>
        <button
            type="button"
            className="gb-btn gb-btn--stay"
            onClick={() => send("game.stay")}
            disabled={!joined || !isMyTurn}
        >Stay</button>
        <button
            type="button"
            className="gb-btn gb-btn--leave"
            onClick={() => send("game.leave")}
            disabled={!joined}
        >Quitter</button>
    </div>
)}
```

- [ ] **Step 7: Commit**

```bash
cd /home/elark/transcendance
git add front-end/src/components/GameBoard.jsx
git commit -m "feat(frontend): skip bet on timer expiry — sit out as spectator"
```

---

### Task 4: Frontend — InlineChat component + 3-column layout

**Files:**
- Modify: `front-end/src/components/GameBoard.jsx`
- Modify: `front-end/src/styles/GameBoard.css`

- [ ] **Step 1: Add `chatMessages` state to `GameBoard`**

After the `skippedRef` declaration added in Task 3:

```js
const [chatMessages, setChatMessages] = useState([])
```

- [ ] **Step 2: Reset `chatMessages` on WS reconnect**

In the same reset block updated in Task 3 Step 2, add `setChatMessages([])`:

```js
setJoined(false); setGameInfo(null); setSelfId(null)
selfIdRef.current = null; setRoundEnd(null); setRevealSeq(null)
skippedRef.current = false; setSkippedThisRound(false); setChatMessages([])
clearRevealTimers()
setStatus(`Connexion à la table ${tableId}…`)
```

- [ ] **Step 3: Send `chat.join` on WS open and `chat.leave` on cleanup**

Replace `socket.onopen` (lines ~310-313):

```js
socket.onopen = () => {
    socket.send(JSON.stringify({ kind: "game.join", payload: { table_id: Number(tableId) } }))
    const username = localStorage.getItem("username") ?? "Joueur"
    socket.send(JSON.stringify({
        kind: "chat.join",
        payload: { room_id: `table-${tableId}`, name: username },
    }))
}
```

Replace the cleanup `return` at the bottom of the WS effect (lines ~445-449):

```js
return () => {
    cancelled = true
    if (socket) {
        if (socket.readyState === WebSocket.OPEN) {
            socket.send(JSON.stringify({
                kind: "chat.leave",
                payload: { room_id: `table-${tableId}` },
            }))
        }
        socket.close()
        wsRef.current = null
    }
    clearRevealTimers()
}
```

Note: the cleanup closure captures the current `tableId` value (the one the effect ran with), so `chat.leave` is sent for the correct room when the user switches tables.

- [ ] **Step 4: Handle incoming `chat.*` messages in `socket.onmessage`**

At the very top of `socket.onmessage`, before the `game.joined` check, add:

```js
socket.onmessage = ({ data }) => {
    let msg
    try { msg = JSON.parse(data) } catch { return }

    const kind    = msg.kind
    const payload = msg.payload ?? msg

    // ── chat messages ─────────────────────────────────────────────────────
    if (kind?.startsWith("chat.")) {
        setChatMessages(prev => [...prev, msg])
        return
    }

    // ── joined ────────────────────────────────────────────────────────────
    if (kind === "game.joined") {
        // ... rest of existing handler unchanged
```

- [ ] **Step 5: Add the `InlineChat` sub-component**

Add this function definition just before `export default function GameBoard()` (around line 246):

```jsx
function InlineChat({ wsRef, tableId, messages }) {
    const [draft, setDraft] = useState("")
    const scrollRef = useRef(null)

    useEffect(() => {
        scrollRef.current?.scrollIntoView({ behavior: "smooth", block: "end" })
    }, [messages])

    const sendMessage = () => {
        const msg = draft.trim()
        if (!msg || wsRef.current?.readyState !== WebSocket.OPEN) return
        wsRef.current.send(JSON.stringify({
            kind: "chat.message",
            payload: { room_id: `table-${tableId}`, message: msg },
        }))
        setDraft("")
    }

    const handleKeyDown = (e) => {
        if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault()
            sendMessage()
        }
    }

    return (
        <aside className="gb-chat">
            <div className="gb-chat__header">
                <span className="gb-chat__title">Chat · Table {tableId}</span>
            </div>
            <div className="gb-chat__messages">
                {messages.length === 0 && (
                    <p className="gb-chat__empty">Aucun message</p>
                )}
                {messages.map((msg, i) => {
                    const envelope = msg.payload ?? msg
                    const isSystem = msg.kind === "chat.system" || envelope.kind === "chat.system"
                    return (
                        <div
                            key={`${envelope.timestamp ?? i}-${i}`}
                            className={`gb-chat__msg${isSystem ? " gb-chat__msg--system" : ""}`}
                        >
                            {!isSystem && (
                                <strong className="gb-chat__author">{envelope.user}</strong>
                            )}
                            <p>{envelope.message}</p>
                        </div>
                    )
                })}
                <div ref={scrollRef} />
            </div>
            <form
                className="gb-chat__composer"
                onSubmit={(e) => { e.preventDefault(); sendMessage() }}
            >
                <textarea
                    className="gb-chat__input"
                    rows={2}
                    value={draft}
                    onChange={e => setDraft(e.target.value)}
                    onKeyDown={handleKeyDown}
                    placeholder="Message…"
                />
                <button type="submit" className="gb-chat__send" disabled={!draft.trim()}>
                    ↵
                </button>
            </form>
        </aside>
    )
}
```

- [ ] **Step 6: Add `InlineChat` to the `GameBoard` render output**

In the `return (...)` of `GameBoard`, after the closing `</div>` of `.gb-board` (around line 707), add:

```jsx
return (
    <div className="gb-layout">

        {/* ── sidebar ─────────────────────────────────────────────────────── */}
        <aside className="gb-sidebar">
            {/* existing sidebar content — unchanged */}
        </aside>

        {/* ── main board ──────────────────────────────────────────────────── */}
        <div className="gb-board">
            {/* existing board content — unchanged */}
        </div>

        {/* ── chat ────────────────────────────────────────────────────────── */}
        <InlineChat wsRef={wsRef} tableId={tableId} messages={chatMessages} />

    </div>
)
```

- [ ] **Step 7: Update `GameBoard.css` — 3-column layout and chat styles**

Replace the `.gb-layout` rule at the top of the file:

```css
.gb-layout {
    display: grid;
    grid-template-columns: 220px minmax(0, 1fr) 280px;
    gap: 24px;
    width: 100%;
    min-height: calc(100vh - 104px);
    align-items: start;
}
```

Add the following new block after the `.gb-sidebar__coins` rule (after the sidebar section):

```css
/* ─── Chat column ─────────────────────────────────────────────────────────── */
.gb-chat {
    display: flex;
    flex-direction: column;
    border-radius: 12px;
    background: rgba(10, 16, 14, 0.95);
    border: 1px solid rgba(255, 255, 255, 0.06);
    position: sticky;
    top: 24px;
    max-height: calc(100vh - 104px);
    overflow: hidden;
}

.gb-chat__header {
    padding: 16px 16px 12px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    flex-shrink: 0;
}

.gb-chat__title {
    font-size: 0.82rem;
    font-weight: 700;
    color: rgba(242, 196, 97, 0.9);
    text-transform: uppercase;
    letter-spacing: 0.12em;
}

.gb-chat__messages {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 200px;
}

.gb-chat__empty {
    margin: auto;
    color: rgba(240, 245, 242, 0.35);
    font-size: 0.82rem;
    text-align: center;
    padding: 24px 0;
}

.gb-chat__msg {
    display: flex;
    flex-direction: column;
    gap: 2px;
}

.gb-chat__msg--system {
    text-align: center;
    color: rgba(240, 245, 242, 0.4);
    font-size: 0.76rem;
    padding: 4px 0;
}

.gb-chat__author {
    font-size: 0.72rem;
    color: rgba(242, 196, 97, 0.8);
    font-weight: 700;
}

.gb-chat__msg p {
    margin: 0;
    font-size: 0.84rem;
    line-height: 1.5;
    word-break: break-word;
    color: rgba(240, 245, 242, 0.88);
}

.gb-chat__composer {
    display: flex;
    gap: 8px;
    padding: 10px 12px;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    flex-shrink: 0;
    align-items: flex-end;
}

.gb-chat__input {
    flex: 1;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    color: var(--text);
    padding: 8px 10px;
    font-size: 0.86rem;
    resize: none;
    outline: none;
    transition: border-color 0.15s;
    font-family: inherit;
}

.gb-chat__input:focus {
    border-color: rgba(196, 168, 130, 0.4);
}

.gb-chat__send {
    width: 36px;
    height: 36px;
    border: 0;
    border-radius: 8px;
    background: var(--accent);
    color: #1a1408;
    font-size: 1.1rem;
    font-weight: 800;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transition: opacity 0.15s;
}

.gb-chat__send:disabled {
    opacity: 0.3;
    cursor: not-allowed;
}

.gb-chat__send:hover:not(:disabled) {
    opacity: 0.88;
}
```

Replace the entire `@media (max-width: 1100px)` block at the bottom of the file:

```css
@media (max-width: 1100px) {
    .gb-layout {
        grid-template-columns: 1fr;
    }

    .gb-sidebar {
        position: static;
        order: 2;
    }

    .gb-board {
        order: 1;
    }

    .gb-chat {
        position: static;
        max-height: 360px;
        order: 3;
    }

    .gb-board__header {
        flex-direction: column;
        align-items: flex-start;
    }

    .gb-arena {
        min-height: 920px;
    }

    .gb-seat--left,
    .gb-seat--right,
    .gb-seat--bottom {
        position: relative;
        inset: auto;
        transform: none;
        width: 100%;
        margin-top: 14px;
    }

    .gb-seats {
        position: relative;
        inset: auto;
        display: grid;
        gap: 14px;
        margin-top: 190px;
        padding: 0 16px 16px;
    }

    .gb-center {
        inset: 210px auto auto 50%;
        width: min(68vw, 320px);
    }

    .outcome-overlay {
        position: fixed;
        border-radius: 0;
    }
}
```

- [ ] **Step 8: Commit**

```bash
cd /home/elark/transcendance
git add front-end/src/components/GameBoard.jsx front-end/src/styles/GameBoard.css
git commit -m "feat(frontend): embed per-table chat as third column in game board"
```

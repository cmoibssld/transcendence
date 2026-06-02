import { useEffect, useRef, useState, useCallback } from "react"
import { apiFetch, getFreshToken } from "../auth"

import "../styles/GameBoard.css"

const GAME_URL = import.meta.env.VITE_GAME_URL || "https://localhost:8443/api/game"
const WS_URL   = import.meta.env.VITE_WS_URL   || "wss://localhost:8443/ws"
const AUTH_URL = import.meta.env.VITE_AUTH_URL  || "https://localhost:8443"

const CARD_IMAGES = import.meta.glob("../assets/cards/*.png", { eager: true, import: "default" })

const SUIT_MAP  = { 1: "hearts", 2: "spades", 3: "diamonds", 4: "clubs" }
const VALUE_MAP = { 1: "A", 11: "J", 12: "Q", 13: "K" }

function cardFileName(card) {
  if (!card || typeof card.value !== "number") return "card_empty.png"
  const suit  = SUIT_MAP[card.color]
  const value = VALUE_MAP[card.value] ?? String(card.value).padStart(2, "0")
  return suit ? `card_${suit}_${value}.png` : "card_empty.png"
}

function cardSrc(fileName) {
  return CARD_IMAGES[`../assets/cards/${fileName}`]
}

function singleCardValue(card) {
  if (!card) return 0
  if (card.value === 1)   return 11
  if (card.value >= 10)  return 10
  return card.value
}

function handValue(cards) {
  let total = 0, aces = 0
  for (const c of (cards || [])) {
    if (!c) continue
    if (c.value === 1)       aces++
    else if (c.value >= 11) total += 10
    else                     total += c.value
  }
  total += aces
  if (aces > 0 && total + 10 <= 21) total += 10
  return total
}

// ─── Dealer CSS avatar ────────────────────────────────────────────────────────
function DealerAvatar() {
  return (
    <div className="gb-dealer-avatar-css" aria-hidden="true">
      <svg viewBox="0 0 48 48" fill="none" xmlns="http://www.w3.org/2000/svg">
        <circle cx="24" cy="16" r="9" fill="#2a2a34" stroke="rgba(196,168,130,0.3)" strokeWidth="1"/>
        <path d="M10 44 Q12 30 24 28 Q36 30 38 44" fill="#1e1e28" stroke="rgba(196,168,130,0.2)" strokeWidth="1"/>
        <path d="M20 29.5 L24 31.5 L28 29.5 L24 27.5 Z" fill="#c4a882" opacity="0.85"/>
        <circle cx="21" cy="15" r="1.2" fill="#c4a882" opacity="0.7"/>
        <circle cx="27" cy="15" r="1.2" fill="#c4a882" opacity="0.7"/>
      </svg>
    </div>
  )
}

// ─── CardImage with flip animation ───────────────────────────────────────────
function CardImage({ card, hidden, flipping }) {
  const [displayFront, setDisplayFront] = useState(!hidden)

  useEffect(() => {
    if (!flipping) { setDisplayFront(!hidden); return }
    const t = setTimeout(() => setDisplayFront(true), 700)
    return () => clearTimeout(t)
  }, [hidden, flipping])

  const src = cardSrc(displayFront ? cardFileName(card) : "card_back.png")
  return (
    <img
      className={`card-image${flipping ? " card-image--flip" : ""}`}
      src={src}
      alt={displayFront ? cardFileName(card) : "Carte cachée"}
    />
  )
}

// ─── Hand ─────────────────────────────────────────────────────────────────────
function Hand({ cards = [], hiddenIndex, flippingIndex, points, label, isActive }) {
  return (
    <div className={`hand${isActive ? " hand--active" : ""}`}>
      <div className="hand__meta">
        <span className="hand__label">{label}</span>
        {typeof points === "number" && <span className="hand__pts">{points} pts</span>}
      </div>
      <div className="hand__cards">
        {cards.length === 0
          ? <span className="hand__empty">–</span>
          : cards.map((card, i) => (
            <div key={card ? `${card.color}-${card.value}-${i}` : i} className="card-slot" style={{ "--delay": `${i * 120}ms` }}>
              <CardImage card={card} hidden={i === hiddenIndex} flipping={i === flippingIndex} />
            </div>
          ))
        }
      </div>
    </div>
  )
}

// ─── Outcome overlay ──────────────────────────────────────────────────────────
const OUTCOME_CONFIG = {
  bj:      { title: "Blackjack !",  cls: "outcome--bj"   },
  win:     { title: "Victoire",     cls: "outcome--win"  },
  lose:    { title: "Défaite",      cls: "outcome--lose" },
  neutral: { title: "Égalité",      cls: "outcome--tie"  },
}

function OutcomeScreen({ roundEnd, myPoints, onClose }) {
  const key = roundEnd.isBlackjack ? "bj" : roundEnd.isPush ? "neutral" : roundEnd.outcomeClass
  const cfg = OUTCOME_CONFIG[key] ?? OUTCOME_CONFIG.neutral
  const { coinDelta, dealerPoints, dealerBlackjack, allWinners } = roundEnd

  return (
    <div className={`outcome-overlay ${cfg.cls}`} role="dialog" aria-modal="true">
      <div className="outcome-card">
        <h2 className="outcome-title">{cfg.title}</h2>

        {roundEnd.isPush && (
          <p className="outcome-push-label">Mise remboursée</p>
        )}

        {dealerBlackjack && !roundEnd.isPush && (
          <p className="outcome-push-label">Croupier Blackjack</p>
        )}

        {roundEnd.isBlackjack && (
          <p className="outcome-bj-badge">🃏 3 pour 2</p>
        )}

        {coinDelta !== 0 && (
          <div className={`outcome-coins ${coinDelta > 0 ? "outcome-coins--win" : "outcome-coins--lose"}`}>
            {coinDelta > 0 ? `+${coinDelta}` : coinDelta} coins
          </div>
        )}

        <div className="outcome-scores">
          <div className="outcome-scores__item">
            <span>Vous</span>
            <strong>{typeof myPoints === "number" ? `${myPoints} pts` : "–"}</strong>
          </div>
          <div className="outcome-scores__sep">VS</div>
          <div className="outcome-scores__item">
            <span>Croupier</span>
            <strong>{dealerPoints} pts</strong>
          </div>
        </div>

        {allWinners.length > 1 && (
          <p className="outcome-detail">Gagnants : Joueur {allWinners.join(", Joueur ")}</p>
        )}

        <button className="outcome-btn" type="button" onClick={onClose}>
          Nouvelle manche
        </button>
      </div>
    </div>
  )
}

// ─── Betting panel ────────────────────────────────────────────────────────────
const BET_PRESETS = [10, 25, 50, 100, 250]
const BET_TIMER_MAX = 30

function TimerRing({ seconds, max, urgent = 10 }) {
  const pct  = Math.max(0, seconds / max) * 100
  const low  = seconds <= urgent
  return (
    <div
      className={`timer-ring${low ? " timer-ring--urgent" : ""}`}
      style={{ "--pct": pct }}
      aria-label={`${seconds}s`}
    >
      <span className="timer-ring__count">{seconds}</span>
    </div>
  )
}

function BettingPanel({ coins, selfPendingBet, onBet, timer }) {
  const [amount, setAmount] = useState(Math.min(10, coins))

  const clamp = (v) => Math.min(Math.max(v, 1), coins)

  if (!selfPendingBet) {
    return (
      <div className="bet-panel bet-panel--waiting">
        <span className="bet-panel__msg">Mise placée — en attente des autres joueurs…</span>
      </div>
    )
  }

  return (
    <div className="bet-panel">
      <div className="bet-panel__header">
        <div className="bet-panel__balance">
          <span className="bet-panel__balance-label">Solde</span>
          <strong className="bet-panel__balance-value">
            {coins.toLocaleString()} <em>coins</em>
          </strong>
        </div>
        {timer !== null && (
          <TimerRing seconds={timer} max={BET_TIMER_MAX} />
        )}
      </div>

      <div className="bet-panel__presets">
        {BET_PRESETS.filter(p => p <= coins).map(p => (
          <button
            key={p}
            type="button"
            className={`bet-chip${amount === p ? " bet-chip--active" : ""}`}
            onClick={() => setAmount(p)}
          >
            <span className="bet-chip__value">{p}</span>
          </button>
        ))}
      </div>

      <div className="bet-panel__row">
        <div className="bet-input-wrap">
          <input
            type="number"
            className="bet-input"
            min={1}
            max={coins}
            value={amount}
            onChange={e => setAmount(clamp(Number(e.target.value)))}
          />
          <span className="bet-input-unit">coins</span>
        </div>
        <button
          type="button"
          className="bet-submit"
          disabled={amount < 1 || amount > coins}
          onClick={() => onBet(amount)}
        >
          Miser
        </button>
      </div>
    </div>
  )
}

// ─── Inline Chat ─────────────────────────────────────────────────────────────
function InlineChat({ wsRef, tableId, messages }) {
  const [draft, setDraft] = useState("")
  const scrollRef = useRef(null)

  useEffect(() => {
    scrollRef.current?.scrollIntoView({ behavior: "smooth", block: "end" })
  }, [messages])

  const sendMessage = () => {
    const msg = draft.trim()
    if (!msg || wsRef.current?.readyState !== WebSocket.OPEN) return
    if (msg.length > 500) {
      // Message too long — block silently (maxLength on textarea prevents this in normal use)
      return
    }
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
      <div className="gb-chat__messages" role="log" aria-live="polite">
        {messages.length === 0 && (
          <p className="gb-chat__empty">Aucun message</p>
        )}
        {messages.map((msg, i) => {
          const envelope = msg.payload ?? msg
          const isSystem = msg.kind === "chat.system" || envelope.kind === "chat.system"
          return (
            <div
              key={msg._id ?? i}
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
          aria-label="Votre message"
          maxLength={500}
        />
        <button type="submit" className="gb-chat__send" disabled={!draft.trim()} aria-label="Envoyer">
          ↵
        </button>
      </form>
    </aside>
  )
}

// ─── Main component ───────────────────────────────────────────────────────────
export default function GameBoard() {
  const [tables,    setTables]    = useState([])
  const [tableId,   setTableId]   = useState("1")
  const [gameInfo,  setGameInfo]  = useState(null)
  const [selfId,    setSelfId]    = useState(null)
  const [status,    setStatus]    = useState("Connexion en attente…")
  const [joined,    setJoined]    = useState(false)
  const [roundEnd,     setRoundEnd]     = useState(null)
  const [coins,        setCoins]        = useState(null)
  const [bettingTimer, setBettingTimer] = useState(null)
  const [turnTimer,    setTurnTimer]    = useState(null)
  const [skippedThisRound, setSkippedThisRound] = useState(false)
  const skippedRef            = useRef(false)
  const [chatMessages, setChatMessages] = useState([])
  const [playerNames,  setPlayerNames]  = useState({}) // id → username
  // Reveal state: null = idle, { cards, flipping } = animating
  const [revealSeq, setRevealSeq] = useState(null)

  // reconnectKey increments to trigger a new connection attempt (never resets to avoid loops)
  // reconnectAttempts is a ref so resetting it on success never causes a re-render
  const [reconnectKey, setReconnectKey]       = useState(0)
  const reconnectAttempts                      = useRef(0)

  const wsRef              = useRef(null)
  const selfIdRef          = useRef(null)
  const revealTimers       = useRef([])
  const intentionalClose   = useRef(false)
  const reconnectTimerRef  = useRef(null)

  const clearRevealTimers = useCallback(() => {
    revealTimers.current.forEach(clearTimeout)
    revealTimers.current = []
  }, [])

  // ── fetch coin balance ──────────────────────────────────────────────────────
  const fetchCoins = useCallback(() => {
    apiFetch(`${AUTH_URL}/auth/balance`)
      .then(r => r.ok ? r.json() : null)
      .then(d => { if (d?.coins !== undefined) setCoins(d.coins) })
      .catch(() => {})
  }, [])

  // ── fetch tables ────────────────────────────────────────────────────────────
  useEffect(() => {
    let alive = true
    fetch(`${GAME_URL}/tables`)
      .then(r => r.ok ? r.json() : [])
      .then(data => { if (alive) setTables(data) })
      .catch(() => {})
    return () => { alive = false }
  }, [tableId])

  // ── WebSocket ───────────────────────────────────────────────────────────────
  useEffect(() => {
    let cancelled = false
    let socket = null

    intentionalClose.current = false
    clearTimeout(reconnectTimerRef.current)

    ;(async () => {
      if (!localStorage.getItem("user_id")) {
        setStatus("Non authentifié — connectez-vous d'abord.")
        return
      }
      if (cancelled) return

      // Fetch coins before connecting — needed to decide if the player can join
      const freshCoins = await apiFetch(`${AUTH_URL}/auth/balance`)
        .then(r => r.ok ? r.json() : null)
        .then(d => d?.coins ?? 0)
        .catch(() => 0)
      setCoins(freshCoins)
      if (cancelled) return

      // Connect without token in URL — browser sends access_token HttpOnly cookie automatically
      socket = new WebSocket(WS_URL)
      wsRef.current = socket

    setJoined(false); setGameInfo(null); setSelfId(null)
    selfIdRef.current = null; setRoundEnd(null); setRevealSeq(null)
    skippedRef.current = false; setSkippedThisRound(false); setChatMessages([])
    clearRevealTimers()
    setStatus(`Connexion à la table ${tableId}…`)

    socket.onopen = () => {
      reconnectAttempts.current = 0
      if (freshCoins <= 0) {
        setStatus("Vous n'avez pas de coins. Impossible de rejoindre la table.")
        return
      }
      socket.send(JSON.stringify({ kind: "game.join", payload: { table_id: Number(tableId) } }))
      const username = localStorage.getItem("username") ?? "Joueur"
      socket.send(JSON.stringify({
        kind: "chat.join",
        payload: { room_id: `table-${tableId}`, name: username },
      }))
    }

    socket.onmessage = ({ data }) => {
      let msg
      try { msg = JSON.parse(data) } catch { return }

      const kind    = msg.kind
      const payload = msg.payload ?? msg

      // ── kicked (same account opened elsewhere) ───────────────────────────
      if (kind === "system.kick") {
        intentionalClose.current = true
        setStatus("⚠️ Compte connecté depuis une autre fenêtre — déconnecté.")
        setJoined(false)
        if (wsRef.current) { wsRef.current.close(); wsRef.current = null }
        return
      }

      // ── chat messages ─────────────────────────────────────────────────────
      if (kind?.startsWith("chat.")) {
        setChatMessages(prev => [...prev, { ...msg, _id: prev.length }])
        return
      }

      // ── joined ────────────────────────────────────────────────────────────
      if (kind === "game.joined") {
        const pid = String(payload.player_id)
        selfIdRef.current = pid
        setSelfId(pid)
        setJoined(true)
        setStatus(`Table ${payload.table_id} — joueur ${pid}`)
        return
      }

      // ── game state ────────────────────────────────────────────────────────
      if (kind === "game.state") {
        // Resolve unknown player names using the server-provided player_id → user_id map
        const userIdMap = payload.player_user_ids ?? {}  // { "player_id": user_id }
        const playerIds = (payload.players_in_game ?? Object.keys(payload.players_cards ?? {})).map(String)
        // Exclude self by player_id (selfIdRef.current is a player_id, same as pid)
        const toFetch = playerIds.filter(pid => pid !== String(selfIdRef.current))
        if (toFetch.length > 0) {
          setPlayerNames(prev => {
            const missing = toFetch.filter(pid => {
              const uid = String(userIdMap[pid])
              return uid && !prev[pid]
            })
            missing.forEach(pid => {
              const uid = userIdMap[pid]
              if (!uid) return
              apiFetch(`${AUTH_URL}/auth/users/byid?id=${uid}`)
                .then(r => r.ok ? r.json() : null)
                .then(data => {
                  if (data?.username) {
                    setPlayerNames(p => ({ ...p, [pid]: data.username }))
                  }
                })
                .catch(() => {})
            })
            return prev
          })
        }
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

      // ── round end ─────────────────────────────────────────────────────────
      if (kind === "game.round_end") {
        const state     = payload.game
        const result    = payload.result
        const dealerPts = result.dealer_points
        const myId      = selfIdRef.current !== null ? Number(selfIdRef.current) : null

        const bjWinnersSet = new Set((result.bj_winners ?? []).map(Number))
        const winnersSet   = new Set((result.winners    ?? []).map(Number))
        const pushesSet    = new Set((result.pushes     ?? []).map(Number))
        const losersSet    = new Set((result.losers     ?? []).map(Number))

        let outcomeClass = "neutral"
        let isBlackjack  = false
        let isPush       = false
        if (myId !== null) {
          if (bjWinnersSet.has(myId))   { outcomeClass = "win"; isBlackjack = true }
          else if (winnersSet.has(myId)) { outcomeClass = "win" }
          else if (pushesSet.has(myId))  { outcomeClass = "neutral"; isPush = true }
          else if (losersSet.has(myId))  { outcomeClass = "lose" }
        }

        const myPoints = myId !== null
          ? (state?.players_points?.[myId] ?? state?.players_points?.[String(myId)])
          : null

        const myPid = selfIdRef.current ? Number(selfIdRef.current) : null
        const myBet = myPid !== null ? (state?.bets?.[myPid] ?? state?.bets?.[String(myPid)] ?? 0) : 0
        const bet   = Number(myBet)
        const coinDelta = isBlackjack ? Math.floor(bet * 3 / 2)
          : outcomeClass === "win"     ? bet
          : outcomeClass === "lose"    ? -bet
          : 0  // push

        setGameInfo(state)

        // Progressive dealer reveal
        const allDealerCards = state?.dealer_cards ?? []
        clearRevealTimers()
        setRevealSeq({ cards: allDealerCards.slice(0, 2), flipping: true })

        const FLIP     = 1600
        const PER_CARD = 900
        const AFTER    = 1000

        revealTimers.current.push(setTimeout(() => {
          setRevealSeq(prev => prev ? { ...prev, flipping: false } : null)
        }, FLIP))

        let t = FLIP
        const extra = allDealerCards.slice(2)
        extra.forEach(card => {
          t += PER_CARD
          revealTimers.current.push(setTimeout(() => {
            setRevealSeq(prev => prev ? { ...prev, cards: [...prev.cards, card] } : null)
          }, t))
        })

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
        return
      }

      // ── waiting ───────────────────────────────────────────────────────────
      if (kind === "game.waiting") {
        setJoined(false)
        setStatus(payload.message || "Aucune table disponible, veuillez patienter.")
        return
      }

      // ── left ──────────────────────────────────────────────────────────────
      if (kind === "game.left") {
        setJoined(false); setGameInfo(null)
        setStatus("Vous avez quitté la table.")
        return
      }

      if (kind === "game.error") {
        setStatus(payload.message || "Erreur de jeu.")
      }
    }

      socket.onclose = () => {
        setJoined(false)
        if (cancelled || intentionalClose.current) {
          setStatus("Connexion fermée.")
          return
        }
        // Unexpected disconnect — reconnect with exponential backoff (1s, 2s, 4s… max 15s)
        const attempt = reconnectAttempts.current
        reconnectAttempts.current = attempt + 1
        const delay = Math.min(1000 * Math.pow(2, attempt), 15000)
        const secs  = Math.round(delay / 1000)
        setStatus(`Connexion perdue. Reconnexion dans ${secs}s…`)
        reconnectTimerRef.current = setTimeout(() => {
          if (!cancelled) setReconnectKey(k => k + 1)
        }, delay)
      }
      socket.onerror = () => {
        if (!intentionalClose.current) setStatus("Erreur WebSocket — reconnexion en cours…")
      }
    })()

    return () => {
      cancelled = true
      intentionalClose.current = true
      clearTimeout(reconnectTimerRef.current)
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
  }, [tableId, fetchCoins, clearRevealTimers, reconnectKey])

  // ── derived state ───────────────────────────────────────────────────────────
  const isRevealing   = revealSeq !== null
  const dealerCards   = isRevealing ? revealSeq.cards : (gameInfo?.dealer_cards ?? [])
  const dealerPts     = gameInfo?.dealer_points
  const currentPlayer = gameInfo?.current_player
  const playersCards  = gameInfo?.players_cards  ?? {}
  const playersPoints = gameInfo?.players_points ?? {}
  const bets          = gameInfo?.bets           ?? {}
  const pendingBets   = gameInfo?.pending_bets   ?? []
  const phase         = gameInfo?.phase          ?? "Betting"

  const allPlayerIds = Object.keys(playersCards).length > 0
    ? Object.keys(playersCards)
    : (gameInfo?.players_in_game?.map(String) ?? [])

  const isMyTurn = selfId !== null && String(currentPlayer) === String(selfId) && phase === "Playing"

  const selfNumId      = selfId !== null ? Number(selfId) : null
  const selfPendingBet = selfNumId !== null && pendingBets.some(p => Number(p) === selfNumId)

  const selfCards  = selfId !== null ? (playersCards[selfId]  ?? playersCards[Number(selfId)]  ?? []) : []
  const selfPoints = selfId !== null ? (playersPoints[selfId] ?? playersPoints[Number(selfId)] ?? 0)  : 0
  const selfBet    = selfId !== null ? (bets[selfId]          ?? bets[Number(selfId)]          ?? 0)  : 0
  const canDouble  = isMyTurn && selfCards.length === 2 && selfPoints !== 21 && coins !== null && selfBet <= coins

  const holeHidden   = !isRevealing && dealerCards.length > 1 && !roundEnd
  const holeFlipping = isRevealing && revealSeq.flipping

  const displayDealerPts = isRevealing
    ? (revealSeq.flipping
        ? singleCardValue(revealSeq.cards[0])
        : handValue(revealSeq.cards))
    : (holeHidden ? singleCardValue(dealerCards[0]) : dealerPts)

  const send = (kind, payload = {}) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      if (kind === "game.leave") intentionalClose.current = true
      wsRef.current.send(JSON.stringify({ kind, payload }))
    }
  }

  // ── Betting timer (server-driven) ───────────────────────────────────────────
  // bet_phase_started_at is a unix timestamp (seconds) sent in every game.state.
  // All clients compute the same remaining time from this shared origin.
  useEffect(() => {
    if (!joined || phase !== "Betting" || !selfPendingBet || roundEnd !== null) {
      setBettingTimer(null)
      return
    }
    const startedAt = gameInfo?.bet_phase_started_at ?? null
    const remaining = () => {
      if (!startedAt) return BET_TIMER_MAX
      return Math.max(0, BET_TIMER_MAX - Math.floor(Date.now() / 1000 - startedAt))
    }
    setBettingTimer(remaining())
    const id = setInterval(() => {
      const t = remaining()
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
  }, [joined, phase, selfPendingBet, roundEnd, gameInfo?.bet_phase_started_at])

  // ── Turn timer ──────────────────────────────────────────────────────────────
  const TURN_TIMER_MAX = 30
  useEffect(() => {
    if (!isMyTurn) { setTurnTimer(null); return }
    setTurnTimer(TURN_TIMER_MAX)
    const id = setInterval(() => {
      setTurnTimer(t => {
        if (t === null || t <= 1) {
          const ws = wsRef.current
          if (ws?.readyState === WebSocket.OPEN) {
            ws.send(JSON.stringify({ kind: "game.stay", payload: {} }))
          }
          return null
        }
        return t - 1
      })
    }, 1000)
    return () => clearInterval(id)
  }, [isMyTurn])

  const handleBet = (amount) => {
    send("game.bet", { amount })
    fetchCoins()
  }

  const seatLabels = ["left", "bottom", "right"]

  return (
    <div className="gb-layout">

      {/* ── sidebar ─────────────────────────────────────────────────────── */}
      <aside className="gb-sidebar">
        <h3>Tables</h3>
        <p className="gb-sidebar__hint">Sélectionne une table pour la rejoindre.</p>
        <div className="gb-sidebar__list">
          {tables.map(t => (
            <button
              key={t.table_id}
              type="button"
              className={`table-chip${String(t.table_id) === tableId ? " table-chip--active" : ""}`}
              onClick={() => setTableId(String(t.table_id))}
            >
              Table {t.table_id} — {t.players} joueur{t.players !== 1 ? "s" : ""}
              {" — "}
              <span
                className={`table-dot ${t.phase === "Playing" ? "table-dot--playing" : t.open ? "table-dot--open" : "table-dot--closed"}`}
              />
              {t.phase === "Playing" ? "en jeu" : t.open ? "ouverte" : "fermée"}
            </button>
          ))}
        </div>

        {coins !== null && (
          <div className="gb-sidebar__coins">
            <span>Vos coins: </span>
            <strong>{coins}</strong>
          </div>
        )}
      </aside>

      {/* ── main board ──────────────────────────────────────────────────── */}
      <div className="gb-board">

        <header className="gb-board__header">
          <div>
            <p className="gb-eyebrow">Blackjack</p>
            <h2>Table {tableId}</h2>
            <p className="gb-status">{status}</p>
          </div>
          <div className="gb-header-right">
            {coins !== null && (
              <span className="gb-coins-badge">{coins} coins</span>
            )}
            <select
              className="gb-table-select"
              value={tableId}
              onChange={e => setTableId(e.target.value)}
              aria-label="Changer de table"
            >
              {tables.map(t => (
                <option key={t.table_id} value={t.table_id}>
                  Table {t.table_id} ({t.players} joueurs)
                </option>
              ))}
            </select>
          </div>
        </header>

        {/* ── arena ───────────────────────────────────────────────────── */}
        <div className="gb-arena">
          {coins !== null && coins <= 0 && !roundEnd && (
            <div className="outcome-overlay outcome--lose" role="alert">
              <div className="outcome-card">
                <h2 className="outcome-title">Aucun coin</h2>
                <p style={{ color: "var(--text-muted, #aaa)", marginBottom: "1rem" }}>
                  Vous n'avez plus de coins pour miser.
                </p>
                <button className="outcome-btn" type="button" onClick={() => window.location.href = "/"}>
                  Retour à l'accueil
                </button>
              </div>
            </div>
          )}
          {roundEnd && (
            <OutcomeScreen
              roundEnd={roundEnd}
              myPoints={roundEnd.myPoints}
              onClose={() => { setRoundEnd(null); setStatus(`Table ${tableId}`) }}
            />
          )}

          {/* dealer */}
          <div className="gb-dealer-zone">
            <div className="gb-dealer-avatar">
              <DealerAvatar />
            </div>
            <Hand
              label="Croupier"
              cards={dealerCards}
              points={displayDealerPts}
              hiddenIndex={holeHidden ? 1 : undefined}
              flippingIndex={holeFlipping ? 1 : undefined}
            />
          </div>

          {/* center ring */}
          <div className="gb-center">
            <div className="gb-center__ring" />
            <div className="gb-center__text">
              <span>Blackjack</span>
              <strong>{allPlayerIds.length}</strong>
            </div>
          </div>

          {/* player seats */}
          <div className="gb-seats">
            {allPlayerIds.length === 0
              ? <p className="gb-empty">Aucun joueur à cette table.</p>
              : allPlayerIds.map((pid, i) => {
                const label     = seatLabels[i] ?? `seat-${i}`
                const isCurrent = String(pid) === String(currentPlayer) && phase === "Playing"
                const isSelf    = String(pid) === String(selfId)
                const pts       = playersPoints[pid] ?? playersPoints[Number(pid)]
                const cards     = playersCards[pid]  ?? playersCards[Number(pid)] ?? []
                const bet       = bets[pid] ?? bets[Number(pid)]
                const hasBet    = bet !== undefined
                const isPending = pendingBets.some(p => Number(p) === Number(pid))
                return (
                  <div
                    key={pid}
                    className={[
                      "gb-seat",
                      `gb-seat--${label}`,
                      isCurrent ? "gb-seat--current" : "",
                      isSelf    ? "gb-seat--self"    : "",
                    ].join(" ").trim()}
                  >
                    <div className="gb-seat__badge">
                      <span>
                        {isSelf
                          ? (localStorage.getItem("username") ?? `#${pid}`) + " (vous)"
                          : (playerNames[pid] ?? `Joueur #${pid}`)}
                      </span>
                      {hasBet && <em className="bet-tag">Mise : {bet}</em>}
                      {isPending && <em className="pending-tag">mise…</em>}
                      {phase === "Playing" && typeof pts === "number" && (
                        <strong>{pts} pts</strong>
                      )}
                      {isCurrent && <em className="turn-tag">Tour</em>}
                    </div>
                    {phase === "Playing" && (
                      <Hand
                        label={isSelf
                          ? (localStorage.getItem("username") ?? `#${pid}`)
                          : (playerNames[pid] ?? `Joueur #${pid}`)}
                        cards={cards}
                        points={typeof pts === "number" ? pts : undefined}
                        isActive={isCurrent}
                      />
                    )}
                  </div>
                )
              })
            }
          </div>
        </div>

        {/* ── actions / betting ───────────────────────────────────────── */}
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
              className="gb-btn gb-btn--double"
              onClick={() => { send("game.double"); fetchCoins() }}
              disabled={!canDouble}
              title={canDouble ? `Doubler la mise (${selfBet} → ${selfBet * 2})` : "Double indisponible"}
            >Double ×2</button>
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
      </div>
      <InlineChat wsRef={wsRef} tableId={tableId} messages={chatMessages} />
    </div>
  )
}

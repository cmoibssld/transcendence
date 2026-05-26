import { useEffect, useRef, useState, useCallback } from "react"

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
  win:     { title: "Victoire",  cls: "outcome--win"  },
  lose:    { title: "Défaite",   cls: "outcome--lose" },
  neutral: { title: "Égalité",   cls: "outcome--tie"  },
}

function OutcomeScreen({ roundEnd, myPoints, onClose }) {
  const cfg = OUTCOME_CONFIG[roundEnd.outcomeClass] ?? OUTCOME_CONFIG.neutral
  const coinDelta = roundEnd.coinDelta
  return (
    <div className={`outcome-overlay ${cfg.cls}`} role="dialog" aria-modal="true">
      <div className="outcome-card">
        <h2 className="outcome-title">{cfg.title}</h2>

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
            <strong>{roundEnd.dealerPoints} pts</strong>
          </div>
        </div>

        {roundEnd.winners.length > 1 && (
          <p className="outcome-detail">Gagnants : Joueur {roundEnd.winners.join(", Joueur ")}</p>
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

function BettingPanel({ coins, selfPendingBet, onBet }) {
  const [amount, setAmount] = useState(10)

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
      <div className="bet-panel__balance">
        <span className="bet-panel__balance-label">Balance</span>
        <strong className="bet-panel__balance-value">{coins} <span>coins</span></strong>
      </div>

      <div className="bet-panel__presets">
        {BET_PRESETS.filter(p => p <= coins).map(p => (
          <button
            key={p}
            type="button"
            className={`bet-preset${amount === p ? " bet-preset--active" : ""}`}
            onClick={() => setAmount(p)}
          >{p}</button>
        ))}
      </div>

      <div className="bet-panel__row">
        <input
          type="number"
          className="bet-input"
          min={1}
          max={coins}
          value={amount}
          onChange={e => setAmount(clamp(Number(e.target.value)))}
        />
        <button
          type="button"
          className="gb-btn gb-btn--hit bet-panel__submit"
          disabled={amount < 1 || amount > coins}
          onClick={() => onBet(amount)}
        >
          Miser
        </button>
      </div>
    </div>
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
  const [roundEnd,  setRoundEnd]  = useState(null)
  const [coins,     setCoins]     = useState(null)
  // Reveal state: null = idle, { cards, flipping } = animating
  const [revealSeq, setRevealSeq] = useState(null)

  const wsRef         = useRef(null)
  const selfIdRef     = useRef(null)
  const revealTimers  = useRef([])

  const clearRevealTimers = useCallback(() => {
    revealTimers.current.forEach(clearTimeout)
    revealTimers.current = []
  }, [])

  // ── fetch coin balance ──────────────────────────────────────────────────────
  const fetchCoins = useCallback(() => {
    const token = localStorage.getItem("access_token")
    if (!token) return
    fetch(`${AUTH_URL}/auth/balance`, {
      headers: { Authorization: `Bearer ${token}` },
    })
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
    const token = localStorage.getItem("access_token")
    if (!token) { setStatus("Non authentifié — connectez-vous d'abord."); return }

    fetchCoins()

    const url = new URL(WS_URL)
    url.searchParams.set("token", token)
    const socket = new WebSocket(url.toString())
    wsRef.current = socket

    setJoined(false); setGameInfo(null); setSelfId(null)
    selfIdRef.current = null; setRoundEnd(null); setRevealSeq(null)
    clearRevealTimers()
    setStatus(`Connexion à la table ${tableId}…`)

    socket.onopen = () => {
      socket.send(JSON.stringify({ kind: "game.join", payload: { table_id: Number(tableId) } }))
    }

    socket.onmessage = ({ data }) => {
      let msg
      try { msg = JSON.parse(data) } catch { return }

      const kind    = msg.kind
      const payload = msg.payload ?? msg

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
        setGameInfo(payload)
        setJoined(true)
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
        const winnersSet = new Set((result.winners ?? []).map(Number))
        const losersSet  = new Set((result.losers  ?? []).map(Number))

        let outcomeClass = "neutral"
        if (myId !== null) {
          if (winnersSet.has(myId))     outcomeClass = "win"
          else if (losersSet.has(myId)) outcomeClass = "lose"
        }

        const myPoints = myId !== null
          ? (state?.players_points?.[myId] ?? state?.players_points?.[String(myId)])
          : null

        // Coin delta for this player
        const myPid = selfIdRef.current ? Number(selfIdRef.current) : null
        const myBet = myPid !== null ? (state?.bets?.[myPid] ?? state?.bets?.[String(myPid)] ?? 0) : 0
        const coinDelta = outcomeClass === "win" ? Number(myBet) : outcomeClass === "lose" ? -Number(myBet) : 0

        setGameInfo(state) // update player info

        // Progressive dealer reveal
        const allDealerCards = state?.dealer_cards ?? []
        clearRevealTimers()
        setRevealSeq({ cards: allDealerCards.slice(0, 2), flipping: true })

        const FLIP     = 1400
        const PER_CARD = 700
        const AFTER    = 400

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
          setRoundEnd({
            dealerPoints: dealerPts,
            winners:      Array.from(winnersSet),
            losers:       Array.from(losersSet),
            outcomeClass,
            myPoints,
            coinDelta,
          })
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

    socket.onclose = () => { setJoined(false); setStatus("Connexion fermée.") }
    socket.onerror = () => setStatus("Erreur WebSocket.")

    return () => {
      socket.close()
      wsRef.current = null
      clearRevealTimers()
    }
  }, [tableId, fetchCoins, clearRevealTimers])

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

  const holeHidden   = !isRevealing && dealerCards.length > 1 && !roundEnd
  const holeFlipping = isRevealing && revealSeq.flipping

  const displayDealerPts = isRevealing
    ? (revealSeq.flipping
        ? singleCardValue(revealSeq.cards[0])
        : handValue(revealSeq.cards))
    : (holeHidden ? singleCardValue(dealerCards[0]) : dealerPts)

  const send = (kind, payload = {}) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify({ kind, payload }))
    }
  }

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
            <span>Vos coins</span>
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
                      <span>Joueur {pid}{isSelf ? " (vous)" : ""}</span>
                      {hasBet && <em className="bet-tag">Mise : {bet}</em>}
                      {isPending && <em className="pending-tag">mise…</em>}
                      {phase === "Playing" && typeof pts === "number" && (
                        <strong>{pts} pts</strong>
                      )}
                      {isCurrent && <em className="turn-tag">Tour</em>}
                    </div>
                    {phase === "Playing" && (
                      <Hand
                        label={`Joueur ${pid}`}
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
          <BettingPanel
            coins={coins ?? 0}
            selfPendingBet={selfPendingBet}
            onBet={handleBet}
          />
        ) : (
          <div className="gb-actions">
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
      </div>
    </div>
  )
}

import { useEffect, useMemo, useRef, useState } from "react"
import dealerAvatar from "../assets/dealer.png"
import "../styles/GameBoard.css"

const GAME_URL = import.meta.env.VITE_GAME_URL || "http://localhost:8080"
const CARD_IMAGES = import.meta.glob("../assets/cards/*.png", {
  eager: true,
  import: "default",
})

function buildCardKey(card) {
  if (!card || typeof card.value !== "number" || typeof card.color !== "number") {
    return "card_empty.png"
  }

  const suitMap = {
    1: "hearts",
    2: "spades",
    3: "diamonds",
    4: "clubs",
  }

  const valueMap = {
    1: "A",
    11: "J",
    12: "Q",
    13: "K",
  }

  const suit = suitMap[card.color]
  const value = valueMap[card.value] || String(card.value).padStart(2, "0")

  if (!suit) {
    return "card_empty.png"
  }

  return `card_${suit}_${value}.png`
}

function CardImage({ card, hidden = false }) {
  const fileName = hidden ? "card_back.png" : buildCardKey(card)
  const src = CARD_IMAGES[`../assets/cards/${fileName}`]
  const alt = hidden ? "Carte cachée" : `${card?.value ?? "?"} of ${card?.color ?? "?"}`

  return <img className="card-image" src={src} alt={alt} />
}

function CardRow({ title, cards = [], points, highlight = false, current = false, hiddenIndexes = [], staggerMs = 120 }) {
  return (
    <section className={`hand-panel${highlight ? " hand-panel--highlight" : ""}${current ? " hand-panel--current" : ""}`}>
      <div className="hand-panel__header">
        <h3>{title}</h3>
        {typeof points === "number" && <span>{points} pts</span>}
      </div>
      <div className="card-row">
        {cards.length === 0 ? (
          <div className="empty-cards">Aucune carte</div>
        ) : (
          cards.map((card, index) => (
            <div
              key={`${title}-${index}`}
              className="card-slot"
              style={{ "--card-delay": `${index * staggerMs}ms` }}
            >
              <CardImage card={card} hidden={hiddenIndexes.includes(index)} />
            </div>
          ))
        )}
      </div>
    </section>
  )
}

function Seat({ playerId, cards, points, current, highlight, seatLabel }) {
  return (
    <div className={`player-seat player-seat--${seatLabel}${current ? " player-seat--current" : ""}${highlight ? " player-seat--highlight" : ""}`}>
      <div className="player-seat__badge">
        <span>Joueur {playerId}</span>
        <strong>{typeof points === "number" ? `${points} pts` : "0 pt"}</strong>
        {current && <em>Tour actif</em>}
      </div>
      <CardRow
        title={`Joueur ${playerId}`}
        cards={cards}
        points={points}
        highlight={highlight}
        current={current}
      />
    </div>
  )
}

function formatPlayerList(playerIds) {
  if (!playerIds.length) {
    return "Aucun"
  }

  return playerIds.map((playerId) => `Joueur ${playerId}`).join(", ")
}

export default function GameBoard() {
  const urlParams = useMemo(() => new URLSearchParams(window.location.search), [])
  const storedTableId = window.localStorage.getItem("blackjack-table-id")
  const initialTableId = urlParams.get("table_id") || storedTableId || "1"

  const [tables, setTables] = useState([])
  const [selectedTableId, setSelectedTableId] = useState(initialTableId)
  const [gameInfo, setGameInfo] = useState(null)
  const [selfPlayerId, setSelfPlayerId] = useState(null)
  const [status, setStatus] = useState("Connexion en attente...")
  const [roundEnd, setRoundEnd] = useState(null)
  const [joined, setJoined] = useState(false)
  const wsRef = useRef(null)
  const selfPlayerIdRef = useRef(null)

  useEffect(() => {
    window.localStorage.setItem("blackjack-table-id", selectedTableId)
  }, [selectedTableId])

  useEffect(() => {
    let mounted = true

    const fetchTables = async () => {
      try {
        const response = await fetch(`${GAME_URL}/tables`)
        if (!response.ok) return
        const data = await response.json()
        if (!mounted) return
        setTables(data)
        if (data.length > 0 && !data.some((table) => String(table.table_id) === String(selectedTableId))) {
          setSelectedTableId(String(data[0].table_id))
        }
      } catch (error) {
        if (mounted) {
          setStatus("Impossible de charger les tables.")
        }
      }
    }

    fetchTables()
    const intervalId = setInterval(fetchTables, 3000)
    return () => {
      mounted = false
      clearInterval(intervalId)
    }
  }, [selectedTableId])

  useEffect(() => {
    const wsUrl = new URL(GAME_URL)
    wsUrl.protocol = wsUrl.protocol === "https:" ? "wss:" : "ws:"
    wsUrl.pathname = "/ws/"
    wsUrl.searchParams.set("table_id", selectedTableId)

    const socket = new WebSocket(wsUrl.toString())
    wsRef.current = socket
    setJoined(false)
    setGameInfo(null)
    setSelfPlayerId(null)
    selfPlayerIdRef.current = null
    setRoundEnd(null)
    setStatus(`Connexion à la table ${selectedTableId}...`)

    socket.onopen = () => {
      socket.send(JSON.stringify({ action: "join" }))
    }

    socket.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data)

        if (data.status === "round_end" && data.game) {
          setGameInfo(data.game)
          const dealerPoints = data.result?.dealer_points ?? data.game.dealer_points
          const myId = selfPlayerIdRef.current !== null ? Number(selfPlayerIdRef.current) : null
          const winners = new Set((data.result?.winners ?? []).map((value) => Number(value)))
          const losers = new Set((data.result?.losers ?? []).map((value) => Number(value)))

          let outcomeText = "Fin de manche"
          let outcomeClass = "neutral"
          if (myId !== null) {
            if (winners.has(myId)) {
              outcomeText = "Fin de manche: vous avez gagné"
              outcomeClass = "win"
            } else if (losers.has(myId)) {
              outcomeText = "Fin de manche: vous avez perdu"
              outcomeClass = "lose"
            } else {
              outcomeText = "Fin de manche: égalité"
            }
          }

          setRoundEnd({
            dealerPoints,
            winners: Array.from(winners),
            losers: Array.from(losers),
            outcomeText,
            outcomeClass,
          })
          setStatus(`${outcomeText} - croupier: ${dealerPoints} pts`)
          return
        }

        if (data.status === "joined") {
          setJoined(true)
          if (data.player_id !== undefined) {
            const playerId = String(data.player_id)
            selfPlayerIdRef.current = playerId
            setSelfPlayerId(playerId)
          }
          setStatus(`Connecté à la table ${data.table_id ?? selectedTableId}`)
          return
        }

        if (data.status === "waiting") {
          setJoined(false)
          setStatus(data.error || "En attente d'une place.")
          return
        }

        if (data.error) {
          setStatus(data.error)
          return
        }

        if (data.table_id !== undefined) {
          setGameInfo(data)
          setJoined(true)
          setStatus(`Table ${data.table_id} prête`)
        }
      } catch (error) {
        setStatus("Réponse serveur invalide.")
      }
    }

    socket.onclose = () => {
      setJoined(false)
      setStatus("Connexion fermée.")
    }

    socket.onerror = () => {
      setStatus("Erreur websocket.")
    }

    return () => {
      socket.close()
      wsRef.current = null
    }
  }, [selectedTableId])

  const dealerCards = gameInfo?.dealer_cards || []
  const dealerPoints = gameInfo?.dealer_points
  const currentPlayer = gameInfo?.current_player
  const playersCards = gameInfo?.players_cards || {}
  const playersPoints = gameInfo?.players_points || {}
  const playersInGame = (gameInfo?.players_in_game?.length ? gameInfo.players_in_game : Object.keys(playersCards)) || []

  const closeRoundEnd = () => {
    setRoundEnd(null)
    if (gameInfo?.table_id !== undefined) {
      setStatus(`Table ${gameInfo.table_id} prête`)
    }
  }

  const sendAction = (action) => {
    if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify({ action }))
    }
  }

  const dealerHoleCardHidden = dealerCards.length > 1 && typeof dealerPoints === "number" && dealerPoints < 17
  const seatLabels = ["left", "bottom", "right"]
  const isMyTurn = selfPlayerId !== null && String(currentPlayer) === String(selfPlayerId)

  return (
    <div className="game-board-layout">
      <div className="game-board">
        <header className="game-board__header">
          <div>
            <p className="eyebrow">Blackjack</p>
            <h2>Table {selectedTableId}</h2>
            <p className="muted">{status}</p>
          </div>
          <div className="table-selector">
            <label htmlFor="table-select">Table active</label>
            <select id="table-select" value={selectedTableId} onChange={(event) => setSelectedTableId(event.target.value)}>
              {tables.map((table) => (
                <option key={table.table_id} value={table.table_id}>
                  Table {table.table_id} ({table.players} joueurs)
                </option>
              ))}
            </select>
          </div>
        </header>

        {roundEnd ? (
          <section className={`round-end-screen round-end-screen--${roundEnd.outcomeClass}`} aria-live="polite" role="dialog" aria-modal="true">
            <div className="round-end-screen__card">
              <span className="round-end-screen__eyebrow">Fin de manche</span>
              <h3>{roundEnd.outcomeText}</h3>
              <p>
                Croupier: <strong>{roundEnd.dealerPoints} pts</strong>
              </p>
              <div className="round-end-screen__grid">
                <div>
                  <span>Gagnants</span>
                  <strong>{formatPlayerList(roundEnd.winners)}</strong>
                </div>
                <div>
                  <span>Perdants</span>
                  <strong>{formatPlayerList(roundEnd.losers)}</strong>
                </div>
              </div>
              <div className="round-end-screen__actions">
                <button type="button" onClick={closeRoundEnd}>
                  Revenir à la table
                </button>
              </div>
            </div>
          </section>
        ) : null}

        <section className="table-stage">
          <div className="table-arena">
            <div className="dealer-panel">
              <div className="dealer-avatar">
                <img src={dealerAvatar} alt="Dealer" />
              </div>
              <CardRow
                title="Croupier"
                cards={dealerCards}
                points={dealerPoints}
                hiddenIndexes={dealerHoleCardHidden ? [1] : []}
                staggerMs={340}
              />
            </div>

            <div className="table-center">
              <div className="table-center__ring" />
              <div className="table-center__text">
                <span>Blackjack</span>
                <strong>{playersInGame.length} joueur{playersInGame.length > 1 ? "s" : ""}</strong>
              </div>
            </div>

            <div className="player-seats">
              {playersInGame.length === 0 ? (
                <div className="empty-state empty-state--arena">Aucun joueur connecté sur cette table.</div>
              ) : (
                playersInGame.map((playerId, index) => {
                  const seatLabel = seatLabels[index] || `seat-${index}`
                  const isCurrentPlayer = String(playerId) === String(currentPlayer)
                  return (
                    <Seat
                      key={playerId}
                      playerId={playerId}
                      cards={playersCards[playerId] || []}
                      points={playersPoints[playerId]}
                      highlight={String(playerId) === String(selfPlayerId)}
                      current={isCurrentPlayer}
                      seatLabel={seatLabel}
                    />
                  )
                })
              )}
            </div>
          </div>

          <div className="game-actions">
            <button type="button" onClick={() => sendAction("hit")} disabled={!joined || !isMyTurn}>
              Hit
            </button>
            <button type="button" onClick={() => sendAction("stay")} disabled={!joined || !isMyTurn}>
              Stay
            </button>
            <button type="button" onClick={() => sendAction("leave")} disabled={!joined}>
              Leave
            </button>
          </div>
        </section>
      </div>

      <aside className="table-sidebar">
        <div className="table-sidebar__header">
          <h3>Tables disponibles</h3>
          <p>Choisis une table pour la rejoindre et voir les cartes réelles.</p>
        </div>
        <div className="table-sidebar__list">
          {tables.map((table) => (
            <button
              key={table.table_id}
              type="button"
              className={`table-chip${String(table.table_id) === String(selectedTableId) ? " table-chip--active" : ""}`}
              onClick={() => {
                setStatus(`Connexion à la table ${table.table_id}...`)
                setSelectedTableId(String(table.table_id))
              }}
            >
              Table {table.table_id} - {table.players} joueurs - {table.open ? "ouverte" : "fermée"}
            </button>
          ))}
        </div>
      </aside>
    </div>
  )
}
import { useEffect, useRef, useState } from "react"
import { useSearchParams } from "react-router-dom"
import "../styles/Chat.css"

const WS_URL = import.meta.env.VITE_WS_URL ?? "wss://localhost/ws"

const createClientId = () =>
  globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(16).slice(2)}`

const ChatPanel = () => {
  const [searchParams, setSearchParams] = useSearchParams()
  const initialRoom = searchParams.get("room") ?? "table-1"
  const initialName = searchParams.get("name") ?? localStorage.getItem("username") ?? "Guest"

  const [roomDraft, setRoomDraft] = useState(initialRoom)
  const [nameDraft, setNameDraft] = useState(initialName)
  const [roomId, setRoomId] = useState(initialRoom)
  const [name, setName] = useState(initialName)
  const [draft, setDraft] = useState("")
  const [messages, setMessages] = useState([])
  const [status, setStatus] = useState("Disconnected")
  const socketRef = useRef(null)
  const nameRef = useRef(initialName)
  const roomIdRef = useRef(initialRoom)
  const clientIdRef = useRef(null)
  const scrollRef = useRef(null)

  useEffect(() => { clientIdRef.current = createClientId() }, [])
  useEffect(() => { scrollRef.current?.scrollIntoView({ behavior: "smooth", block: "end" }) }, [messages])
  useEffect(() => { nameRef.current = name }, [name])
  useEffect(() => { roomIdRef.current = roomId }, [roomId])

  useEffect(() => {
    const token = localStorage.getItem("access_token")
    if (!token) { setStatus("Non authentifié"); return }

    const url = new URL(WS_URL)
    url.searchParams.set("token", token)

    const socket = new WebSocket(url.toString())
    socketRef.current = socket
    setStatus("Connecting")
    setMessages([])

    socket.addEventListener("open", () => {
      if (!clientIdRef.current) clientIdRef.current = createClientId()
      setStatus("Connected")
      socket.send(JSON.stringify({
        kind: "chat.join",
        payload: { room_id: roomIdRef.current, name: nameRef.current },
      }))
    })

    socket.addEventListener("close", () => setStatus("Disconnected"))
    socket.addEventListener("error", () => setStatus("Connection error"))

    socket.addEventListener("message", (event) => {
      try {
        const data = JSON.parse(event.data)
        // ws-service wraps as {kind, payload}; chat service sends envelope directly
        const envelope = data.kind?.startsWith("chat.") ? data : (data.payload ?? data)
        setMessages((prev) => [...prev, envelope])
      } catch {
        setMessages((prev) => [...prev, {
          kind: "chat.system", user: "system", message: event.data,
          room_id: roomIdRef.current, timestamp: Date.now() / 1000,
        }])
      }
    })

    return () => { socket.close() }
  }, [roomId])

  // Notify server when page is hidden
  useEffect(() => {
    const handlePageHide = () => {
      const s = socketRef.current
      if (s?.readyState === WebSocket.OPEN) {
        s.send(JSON.stringify({ kind: "chat.leave", payload: { room_id: roomIdRef.current } }))
        s.close()
      }
    }
    window.addEventListener("pagehide", handlePageHide)
    return () => window.removeEventListener("pagehide", handlePageHide)
  }, [])

  const handleJoin = (e) => {
    e.preventDefault()
    const nextRoom = roomDraft.trim() || "table-1"
    const nextName = nameDraft.trim() || "Guest"

    if (nextRoom !== roomId) {
      socketRef.current?.readyState === WebSocket.OPEN &&
        socketRef.current.send(JSON.stringify({ kind: "chat.leave", payload: { room_id: roomIdRef.current } }))
      setRoomId(nextRoom)
      setName(nextName)
      nameRef.current = nextName
      setSearchParams({ room: nextRoom, name: nextName })
      return
    }

    setName(nextName)
    nameRef.current = nextName
    setSearchParams({ room: nextRoom, name: nextName })

    if (socketRef.current?.readyState === WebSocket.OPEN) {
      socketRef.current.send(JSON.stringify({
        kind: "chat.rename",
        payload: { room_id: roomIdRef.current, name: nextName },
      }))
    }
  }

  const handleSubmit = (e) => {
    e.preventDefault()
    const message = draft.trim()
    if (!message || socketRef.current?.readyState !== WebSocket.OPEN) return
    socketRef.current.send(JSON.stringify({
      kind: "chat.message",
      payload: { room_id: roomIdRef.current, message },
    }))
    setDraft("")
  }

  return (
    <section className="chat-panel page-card">
      <div className="chat-panel__header">
        <div>
          <span className="section-chip">Blackjack chat</span>
          <h2 className="page-title">Room {roomId}</h2>
          <p className="page-copy">Use the same room ID as the blackjack table to keep the conversation isolated per game.</p>
        </div>
        <div className={`chat-status chat-status--${status.toLowerCase().replace(/\s+/g, "-")}`}>
          {status}
        </div>
      </div>

      <form className="chat-panel__controls" onSubmit={handleJoin}>
        <label className="chat-field">
          <span>Room</span>
          <input value={roomDraft} onChange={(e) => setRoomDraft(e.target.value)} placeholder="table-1" />
        </label>
        <label className="chat-field">
          <span>Name</span>
          <input value={nameDraft} onChange={(e) => setNameDraft(e.target.value)} placeholder="Guest" />
        </label>
        <button type="submit" className="btn-secondary">Join room</button>
      </form>

      <div className="chat-panel__messages" aria-live="polite">
        {messages.length === 0 ? (
          <div className="chat-empty">
            <strong>No messages yet.</strong>
            <span>Join the room and send the first note.</span>
          </div>
        ) : null}

        {messages.map((msg, index) => {
          const envelope = msg.payload ?? msg
          const isSystem = msg.kind === "chat.system" || envelope.kind === "chat.system"
          const msgClientId = envelope.client_id
          const isOwn = !isSystem && msgClientId === clientIdRef.current

          return (
            <article
              key={`${envelope.timestamp}-${index}`}
              className={`chat-message ${isSystem ? "chat-message--system" : isOwn ? "chat-message--own" : ""}`}
            >
              <div className="chat-message__meta">
                <strong>{isSystem ? "System" : isOwn ? "You" : envelope.user}</strong>
                <span>{new Date((envelope.timestamp ?? 0) * 1000).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}</span>
              </div>
              <p>{envelope.message}</p>
            </article>
          )
        })}
        <div ref={scrollRef} />
      </div>

      <form className="chat-panel__composer" onSubmit={handleSubmit}>
        <textarea
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          placeholder="Write a message to your blackjack table"
          rows={3}
        />
        <button type="submit" className="btn-primary">Send message</button>
      </form>
    </section>
  )
}

export default ChatPanel

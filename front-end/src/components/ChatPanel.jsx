import { useEffect, useRef, useState } from "react"
import { useSearchParams } from "react-router-dom"
import "../styles/Chat.css"

const chatBaseUrl = import.meta.env.VITE_CHAT_URL ?? "http://localhost:8090"

const createClientId = () => {
  if (globalThis.crypto?.randomUUID) {
    return globalThis.crypto.randomUUID()
  }

  return `${Date.now()}-${Math.random().toString(16).slice(2)}`
}

const buildSocketUrl = (roomId) => {
  const url = new URL(chatBaseUrl)
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:"
  url.pathname = `/ws/${encodeURIComponent(roomId)}`
  return url.toString()
}

const ChatPanel = () => {
  const [searchParams, setSearchParams] = useSearchParams()
  const initialRoom = searchParams.get("room") ?? "table-1"
  const initialName = searchParams.get("name") ?? "Guest"
  const [roomDraft, setRoomDraft] = useState(initialRoom)
  const [nameDraft, setNameDraft] = useState(initialName)
  const [roomId, setRoomId] = useState(initialRoom)
  const [name, setName] = useState(initialName)
  const [draft, setDraft] = useState("")
  const [messages, setMessages] = useState([])
  const [status, setStatus] = useState("Disconnected")
  const socketRef = useRef(null)
  const nameRef = useRef(initialName)
  const clientIdRef = useRef(null)
  const scrollRef = useRef(null)

  useEffect(() => {
    clientIdRef.current = createClientId()
  }, [])

  useEffect(() => {
    scrollRef.current?.scrollIntoView({ behavior: "smooth", block: "end" })
  }, [messages])

  useEffect(() => {
    nameRef.current = name
  }, [name])

  useEffect(() => {
    const handlePageHide = () => {
      const socket = socketRef.current
      if (socket?.readyState === WebSocket.OPEN) {
        socket.send(JSON.stringify({ action: "leave", client_id: clientIdRef.current, name: nameRef.current }))
        socket.close()
      }
    }

    window.addEventListener("pagehide", handlePageHide)

    return () => {
      window.removeEventListener("pagehide", handlePageHide)
    }
  }, [])

  useEffect(() => {
    const socket = new WebSocket(buildSocketUrl(roomId))
    socketRef.current = socket
    setStatus("Connecting")
    setMessages([])

    socket.addEventListener("open", () => {
      if (!clientIdRef.current) {
        clientIdRef.current = createClientId()
      }
      setStatus("Connected")
      socket.send(JSON.stringify({ action: "join", name: nameRef.current, client_id: clientIdRef.current }))
    })
    socket.addEventListener("close", () => setStatus("Disconnected"))
    socket.addEventListener("error", () => setStatus("Connection error"))
    socket.addEventListener("message", (event) => {
      try {
        const payload = JSON.parse(event.data)
        setMessages((currentMessages) => [...currentMessages, payload])
      } catch {
        setMessages((currentMessages) => [
          ...currentMessages,
          {
            kind: "system",
            user: "system",
            message: event.data,
            room_id: roomId,
            timestamp: Date.now(),
          },
        ])
      }
    })

    return () => {
      socket.close()
    }
  }, [roomId])

  const handleJoin = (event) => {
    event.preventDefault()
    const nextRoomId = roomDraft.trim() || "table-1"
    const nextName = nameDraft.trim() || "Guest"

    if (!clientIdRef.current) {
      clientIdRef.current = createClientId()
    }

    if (nextRoomId !== roomId) {
      if (socketRef.current?.readyState === WebSocket.OPEN) {
        socketRef.current.send(JSON.stringify({ action: "leave", client_id: clientIdRef.current, name: nameRef.current }))
      }
      setRoomId(nextRoomId)
      setName(nextName)
      nameRef.current = nextName
      setSearchParams({ room: nextRoomId, name: nextName })
      return
    }

    setName(nextName)
    nameRef.current = nextName
    setSearchParams({ room: nextRoomId, name: nextName })

    if (socketRef.current?.readyState === WebSocket.OPEN) {
      socketRef.current.send(JSON.stringify({ action: "join", name: nextName, client_id: clientIdRef.current }))
    }
  }

  const handleSubmit = (event) => {
    event.preventDefault()

    const message = draft.trim()
    if (!message || socketRef.current?.readyState !== WebSocket.OPEN) {
      return
    }

    socketRef.current.send(JSON.stringify({ message }))
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
          <input value={roomDraft} onChange={(event) => setRoomDraft(event.target.value)} placeholder="table-1" />
        </label>

        <label className="chat-field">
          <span>Name</span>
          <input value={nameDraft} onChange={(event) => setNameDraft(event.target.value)} placeholder="Guest" />
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

        {messages.map((message, index) => {
          const isSystem = message.kind === "system"
          const isOwnMessage = message.kind === "message" && message.client_id === clientIdRef.current

          return (
            <article
              key={`${message.timestamp}-${index}`}
              className={`chat-message ${isSystem ? "chat-message--system" : isOwnMessage ? "chat-message--own" : ""}`}
            >
              <div className="chat-message__meta">
                <strong>{isSystem ? "System" : isOwnMessage ? "You" : message.user}</strong>
                <span>{new Date(message.timestamp * 1000).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}</span>
              </div>
              <p>{message.message}</p>
            </article>
          )
        })}
        <div ref={scrollRef} />
      </div>

      <form className="chat-panel__composer" onSubmit={handleSubmit}>
        <textarea
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          placeholder="Write a message to your blackjack table"
          rows={3}
        />
        <button type="submit" className="btn-primary">Send message</button>
      </form>
    </section>
  )
}

export default ChatPanel

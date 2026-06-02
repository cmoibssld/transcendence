import { useEffect, useRef, useState } from "react"
import { BrowserRouter as Router, Routes, Route, Navigate, useNavigate, useLocation } from "react-router-dom"
import { apiFetch } from "./auth"
import Header from "./components/Header"
import Footer from "./components/Footer"
import Home from "./pages/Home"
import Profile from "./pages/Profile"
import Friends from "./pages/Friends"
import Rules from "./pages/Rules"
import Game from "./pages/Game"
import Statistics from "./pages/Statistics"
import Settings from "./pages/Settings"
import Login from "./pages/Login"
import Register from "./pages/Register"
import History from "./pages/History"
import PrivacyPolicy from "./pages/PrivacyPolicy"
import TermsOfService from "./pages/TermsOfService"
import "./App.css"

function OAuthCallback() {
  const navigate = useNavigate()
  const location = useLocation()

  useEffect(() => {
    const params = new URLSearchParams(location.search)
    const code = params.get("code")
    if (!code) return

    const authUrl = import.meta.env.VITE_AUTH_URL ?? "https://localhost:8443"
    fetch(`${authUrl}/auth/exchange?code=${encodeURIComponent(code)}`, {
      credentials: "include"
    })
      .then(r => {
        if (!r.ok) throw new Error(`exchange_http_${r.status}`)
        return r.json()
      })
      .then(data => {
        // Tokens are now in HttpOnly cookies — store only display info
        if (data.username || data.user_id) {
          if (data.username)   localStorage.setItem("username",   data.username)
          if (data.avatar_url) localStorage.setItem("avatar_url", data.avatar_url ?? "")
          if (data.user_id)    localStorage.setItem("user_id",    String(data.user_id))
          navigate("/", { replace: true })
        } else {
          navigate(`/login?error=oauth_failed&reason=no_tokens`, { replace: true })
        }
      })
      .catch((err) => {
        console.error("[OAuthCallback] exchange failed:", err)
        navigate(`/login?error=oauth_failed&reason=${encodeURIComponent(err.message ?? "unknown")}`, { replace: true })
      })
  }, [location.search, navigate])

  return null
}

const AUTH_URL = import.meta.env.VITE_AUTH_URL ?? "https://localhost:8443"

function useGlobalPresence() {
  const pingRef = useRef(null)

  useEffect(() => {
    // user_id is stored in localStorage on login (not sensitive — just an integer ID)
    const userId = Number(localStorage.getItem("user_id"))
    if (!userId) return

    const ping = () => {
      // Re-check on each tick so the interval self-cancels after logout
      const currentId = Number(localStorage.getItem("user_id"))
      if (!currentId) {
        clearInterval(pingRef.current)
        return
      }
      fetch(`${AUTH_URL}/auth/presence/online`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({ user_id: currentId }),
      }).catch(() => {})
    }

    ping()
    pingRef.current = setInterval(ping, 4 * 60 * 1000)

    const goOffline = () =>
      fetch(`${AUTH_URL}/auth/presence/offline`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({ user_id: userId }),
        keepalive: true,
      }).catch(() => {})
    window.addEventListener("beforeunload", goOffline)

    return () => {
      clearInterval(pingRef.current)
      window.removeEventListener("beforeunload", goOffline)
    }
  }, [])
}

function ProtectedRoute({ children }) {
  const userId = localStorage.getItem("user_id")
  const navigate = useNavigate()
  // sessionStorage flag avoids a round-trip on every route change within the same tab.
  // It resets automatically when the tab/window is closed.
  const [verified, setVerified] = useState(
    userId !== null && sessionStorage.getItem("auth_ok") === "1"
  )

  useEffect(() => {
    if (!userId) return
    if (sessionStorage.getItem("auth_ok") === "1") return
    apiFetch(`${AUTH_URL}/auth/profile`)
      .then(r => {
        if (r.ok) {
          sessionStorage.setItem("auth_ok", "1")
          setVerified(true)
        } else {
          sessionStorage.removeItem("auth_ok")
          localStorage.removeItem("user_id")
          localStorage.removeItem("username")
          localStorage.removeItem("avatar_url")
          navigate("/login", { replace: true })
        }
      })
      .catch(() => {
        navigate("/login", { replace: true })
      })
  }, [userId, navigate])

  if (!userId) return <Navigate to="/login" replace />
  if (!verified) return null
  return children
}

function App() {
  useGlobalPresence()
  return (
    <Router>
      <div className="app-shell">
        <Header />
        <main className="app-main">
          <Routes>
            <Route path="/" element={<Home />} />
            <Route path="/auth/callback" element={<OAuthCallback />} />
            <Route path="/login" element={<Login />} />
            <Route path="/register" element={<Register />} />
            <Route path="/privacy" element={<PrivacyPolicy />} />
            <Route path="/terms" element={<TermsOfService />} />
            <Route path="/rules" element={<Rules />} />
            <Route path="/profile"    element={<ProtectedRoute><Profile /></ProtectedRoute>} />
            <Route path="/friends"    element={<ProtectedRoute><Friends /></ProtectedRoute>} />
            <Route path="/statistics" element={<ProtectedRoute><Statistics /></ProtectedRoute>} />
            <Route path="/game"       element={<ProtectedRoute><Game /></ProtectedRoute>} />
            <Route path="/settings"   element={<ProtectedRoute><Settings /></ProtectedRoute>} />
            <Route path="/history"    element={<ProtectedRoute><History /></ProtectedRoute>} />
          </Routes>
        </main>
        <Footer />
      </div>
    </Router>
  )
}

export default App

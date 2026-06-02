import { useEffect, useRef } from "react"
import { BrowserRouter as Router, Routes, Route, useNavigate, useLocation } from "react-router-dom"
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

    const ping = () =>
      fetch(`${AUTH_URL}/auth/presence/online`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",  // sends access_token HttpOnly cookie
        body: JSON.stringify({ user_id: userId }),
      }).catch(() => {})

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
            <Route path="/profile" element={<Profile />} />
            <Route path="/friends" element={<Friends />} />
            <Route path="/statistics" element={<Statistics />} />
            <Route path="/rules" element={<Rules />} />
            <Route path="/game" element={<Game />} />
            <Route path="/settings" element={<Settings />} />
            <Route path="/history" element={<History />} />
            <Route path="/privacy" element={<PrivacyPolicy />} />
            <Route path="/terms" element={<TermsOfService />} />
          </Routes>
        </main>
        <Footer />
      </div>
    </Router>
  )
}

export default App

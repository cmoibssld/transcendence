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
    fetch(`${authUrl}/auth/exchange?code=${encodeURIComponent(code)}`)
      .then(r => {
        if (!r.ok) throw new Error(`exchange_http_${r.status}`)
        return r.json()
      })
      .then(data => {
        if (data.access_token && data.refresh_token) {
          localStorage.setItem("access_token", data.access_token)
          localStorage.setItem("refresh_token", data.refresh_token)
          if (data.username) localStorage.setItem("username", data.username)
          if (data.avatar_url) localStorage.setItem("avatar_url", data.avatar_url)
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
    const token = localStorage.getItem("access_token")
    if (!token) return

    // Decode JWT to get user_id (sub field)
    let userId = 0
    try {
      const payload = JSON.parse(atob(token.split(".")[1]))
      userId = payload.sub ?? 0
    } catch { return }
    if (!userId) return

    const ping = () =>
      fetch(`${AUTH_URL}/auth/presence/online`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify({ user_id: userId }),
      }).catch(() => {})

    ping()
    // Refresh every 4 minutes so the in-memory store survives restarts
    pingRef.current = setInterval(ping, 4 * 60 * 1000)

    const goOffline = () =>
      navigator.sendBeacon(
        `${AUTH_URL}/auth/presence/offline`,
        JSON.stringify({ user_id: userId })
      )
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

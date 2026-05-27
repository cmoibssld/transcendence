import { useEffect } from "react"
import { BrowserRouter as Router, Routes, Route, useNavigate, useLocation } from "react-router-dom"
import Header from "./components/Header"
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
import Chat from "./pages/Chat"
import "./App.css"

function OAuthCallback() {
  const navigate = useNavigate()
  const location = useLocation()

  useEffect(() => {
    const params = new URLSearchParams(location.search)
    const code = params.get("code")
    if (!code) return

    const authUrl = import.meta.env.VITE_AUTH_URL ?? "http://localhost:8081"
    fetch(`${authUrl}/auth/exchange?code=${encodeURIComponent(code)}`)
      .then(r => r.json())
      .then(data => {
        if (data.access_token && data.refresh_token) {
          localStorage.setItem("access_token", data.access_token)
          localStorage.setItem("refresh_token", data.refresh_token)
          if (data.username) localStorage.setItem("username", data.username)
          if (data.avatar_url) localStorage.setItem("avatar_url", data.avatar_url)
          navigate("/", { replace: true })
        } else {
          navigate("/login?error=oauth_failed", { replace: true })
        }
      })
      .catch(() => {
        navigate("/login?error=oauth_failed", { replace: true })
      })
  }, [location.search, navigate])

  return null
}

function App() {
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
            <Route path="/chat" element={<Chat />} />
            <Route path="/settings" element={<Settings />} />
            <Route path="/history" element={<History />} />
          </Routes>
        </main>
      </div>
    </Router>
  )
}

export default App

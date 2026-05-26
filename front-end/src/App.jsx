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

// Reads tokens from the GitHub OAuth redirect (?access_token=...&refresh_token=...&username=...)
// and stores them in localStorage, then strips the params from the URL.
function OAuthCallback() {
  const navigate = useNavigate()
  const location = useLocation()

  useEffect(() => {
    const params = new URLSearchParams(location.search)
    const accessToken = params.get("access_token")
    const refreshToken = params.get("refresh_token")
    const username = params.get("username")

    if (accessToken && refreshToken) {
      localStorage.setItem("access_token", accessToken)
      localStorage.setItem("refresh_token", refreshToken)
      if (username) localStorage.setItem("username", username)
      const avatarUrl = params.get("avatar_url")
      if (avatarUrl) localStorage.setItem("avatar_url", avatarUrl)
      // Replace current entry so back-button doesn't re-process the tokens
      navigate("/", { replace: true })
    }
  }, [location.search, navigate])

  return null
}

function App() {
  return (
    <Router>
      <OAuthCallback />
      <div className="app-shell">
        <Header />
        <main className="app-main">
          <Routes>
            <Route path="/" element={<Home />} />
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

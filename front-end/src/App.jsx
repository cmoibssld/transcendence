import { BrowserRouter as Router, Routes, Route } from "react-router-dom"
import Header from "./components/Header"
import Home from "./pages/Home"
import Profile from "./pages/Profile"
import Friends from "./pages/Friends"
import Rules from "./pages/Rules"
import Game from "./pages/Game"
import Statistics from "./pages/Statistics"
import Settings from "./pages/Settings"
import Login from "./pages/Login"
import History from "./pages/History"
import "./App.css"

function App() {
  return (
    <Router>
      <Header />

      <Routes>
        <Route path="/" element={<Home />} />
        <Route path="/login" element={<Login />} />
        <Route path="/profile" element={<Profile />} />
        <Route path="/friends" element={<Friends />} />
        <Route path="/statistics" element={<Statistics />} />
        <Route path="/rules" element={<Rules />} />
        <Route path="/game" element={<Game />} />
        <Route path="/settings" element={<Settings />} />
        <Route path="/history" element={<History />} />
      </Routes>
    </Router>
  )
}

export default App
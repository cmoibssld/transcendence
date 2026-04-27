import '../styles/Header.css'
import logo from '../assets/Alexis.jpg'
import { Link } from "react-router-dom"
import { FaCog } from "react-icons/fa"
import { useEffect, useState } from 'react'

const Header = () => {
  const [player, setPlayer] = useState(null)

  useEffect(() => {
    const loadPlayer = () => {
      const raw = localStorage.getItem("playerProfile")
      if (!raw) {
        setPlayer(null)
        return
      }

      try {
        setPlayer(JSON.parse(raw))
      } catch {
        setPlayer(null)
      }
    }

    loadPlayer()
    window.addEventListener("storage", loadPlayer)
    window.addEventListener("player-updated", loadPlayer)

    return () => {
      window.removeEventListener("storage", loadPlayer)
      window.removeEventListener("player-updated", loadPlayer)
    }
  }, [])

  const avatarSrc = player?.avatar && player.avatar.startsWith("http") ? player.avatar : logo

  return (
    <header className="header">
        <img src={avatarSrc} alt="avatar" className="header-avatar" />
      <nav className="nav">
        <Link to="/profile">{player?.username || "BossLady"}</Link>
        <Link to="/">HOME</Link>
        <Link to="/friends">FRIENDS</Link>
        <Link to="/statistics">STATISTICS</Link>
        <Link to="/history">GAME HISTORY</Link>
        <Link to="/rules">RULES</Link>
      </nav>
        <div className="user-info">
          <span className="username">Coins: {player?.money ?? "--"}</span>
          <span className="username">Avatar: {player?.avatar || "default"}</span>
        </div>
        <Link to="/settings">
          <FaCog className="icon" />
        </Link>
    </header>

  )
}

export default Header

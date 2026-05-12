import '../styles/Header.css'
import logo from '../assets/JB.jpg'
import { Link } from "react-router-dom"
import { FaCog } from "react-icons/fa"

const Header = () => {
  return (
    <header className="header">
      <div className="header-brand">
        <img src={logo} alt="avatar" className="header-avatar" />
        <div className="brand-copy">
          <span className="brand-kicker">Luigi Jack</span>
          <Link to="/profile" className="brand-name">BossLady</Link>
        </div>
      </div>

      <nav className="nav">
        <Link to="/">Home</Link>
        <Link to="/friends">Friends</Link>
        <Link to="/statistics">Stats</Link>
        <Link to="/history">History</Link>
        <Link to="/rules">Rules</Link>
      </nav>

      <Link to="/settings" className="settings-link" aria-label="Settings">
        <FaCog className="icon" />
      </Link>
    </header>

  )
}

export default Header
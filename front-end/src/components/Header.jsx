import '../styles/Header.css'
import logo from '../assets/JB.jpg'
import { Link } from "react-router-dom"
import { FaCog } from "react-icons/fa"

const Header = () => {
  return (
    <header className="header">
        <img src={logo} alt="avatar" className="header-avatar" />
      <nav className="nav">
        <Link to="/profile">BossLady</Link>
        <Link to="/">HOME</Link>
        <Link to="/friends">FRIENDS</Link>
        <Link to="/statistics">STATISTICS</Link>
        <Link to="/history">GAME HISTORY</Link>
        <Link to="/rules">RULES</Link>
      </nav>
        <Link to="/settings">
          <FaCog className="icon" />
        </Link>
    </header>

  )
}

export default Header
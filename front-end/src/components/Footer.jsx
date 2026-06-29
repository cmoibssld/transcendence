import { Link } from "react-router-dom"
import "../styles/Footer.css"

export default function Footer() {
  return (
    <footer className="app-footer">
      <div className="footer-inner">
        <span className="footer-brand">Luigi Jack</span>
        <nav className="footer-links">
          <Link to="/privacy">Confidentialité</Link>
          <Link to="/terms">CGU</Link>
          <Link to="/rules">Règles du jeu</Link>
        </nav>
        <span className="footer-copy">© 2026 — Projet 42</span>
      </div>
    </footer>
  )
}

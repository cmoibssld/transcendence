import "../styles/Home.css"
import { Link } from "react-router-dom"
import heroImage from "../assets/background.png"

const Home = () => {
  return (
    <div className="home-page">
      <img src={heroImage} alt="Casino table" className="home-bg" />
      <div className="home-overlay" />

      <section className="home-hero">
        <div className="home-eyebrow">Multijoueur · En ligne</div>
        <h1 className="home-title">Black<span>Jack</span></h1>
        <p className="home-subtitle">
          Rejoins une table, bats le croupier et prouve que tu as la meilleure main.
        </p>
        <div className="home-actions">
          <Link to="/game" className="btn-primary home-btn-play">Jouer maintenant</Link>
          <Link to="/rules" className="btn-secondary">Règles du jeu</Link>
        </div>
      </section>

      <div className="home-cards-deco" aria-hidden="true">
        <span className="deco-card deco-card--1">A♠</span>
        <span className="deco-card deco-card--2">K♥</span>
        <span className="deco-card deco-card--3">J♦</span>
      </div>
    </div>
  )
}

export default Home

import "../styles/Home.css"
import { Link } from "react-router-dom";
import heroImage from "../assets/background.png";
import titleGame from "../assets/game_name.png";

const Home = () => {
  return (
    <div className="home-page">
      <img src={heroImage} alt="Casino table" className="home-background" />
      <div className="home-overlay" />

      <section className="home-panel">
        <img src={titleGame} alt="Luigi-Jack" className="game-title"/>
        <div className="home-actions">
          <Link to="/game" className="btn-primary">Play now</Link>
          <Link to="/rules" className="btn-secondary">Read rules</Link>
        </div>
      </section>
    </div>
  )
}

export default Home
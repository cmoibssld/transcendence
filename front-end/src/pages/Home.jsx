import "../styles/Home.css"
import Table from "../components/Table"
import Dealer from "../components/Dealer"
import { Link } from "react-router-dom";
import playButton from "../assets/play-button.png";
import titleGame from "../assets/game_name.png";

const Home = () => {
  return (
    <div className="game-area">
      <Dealer />
      <Table />
      <img src={titleGame} alt="Luigi-Jack" className="game-title"/>
      <Link to="/game" className="play-link">
        <img src={playButton} alt="Play" className="play-button"
        />
      </Link>
    </div>
  )
}

export default Home
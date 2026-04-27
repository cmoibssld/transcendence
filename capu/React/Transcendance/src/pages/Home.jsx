import "../styles/Home.css"
import Table from "../components/Table"
import Dealer from "../components/Dealer"
import { Link } from "react-router-dom";
import { useEffect, useState } from "react";
import playButton from "../assets/play-button.png";
import titleGame from "../assets/game_name.png";
import { fetchPlayerById } from "../services/playerApi";

const Home = () => {
  const [playerIdInput, setPlayerIdInput] = useState(() => localStorage.getItem("playerId") || "1");
  const [player, setPlayer] = useState(null);
  const [error, setError] = useState("");

  const loadPlayer = async (playerId) => {
    try {
      setError("");
      const result = await fetchPlayerById(playerId);
      setPlayer(result);
      localStorage.setItem("playerId", String(result.id));
      localStorage.setItem("playerProfile", JSON.stringify(result));
      window.dispatchEvent(new CustomEvent("player-updated"));
    } catch (_err) {
      setPlayer(null);
      setError("Impossible de charger ce joueur depuis la DB");
    }
  };

  useEffect(() => {
    if (!playerIdInput) {
      return;
    }
    loadPlayer(playerIdInput);
  }, []);

  return (
    <div className="game-area">
      <Dealer />
      <Table />
      <img src={titleGame} alt="Luigi-Jack" className="game-title"/>

      <div className="player-boot-card">
        <h3>Profil DB au lancement</h3>
        <div className="player-boot-controls">
          <input
            type="number"
            min="1"
            value={playerIdInput}
            onChange={(event) => setPlayerIdInput(event.target.value)}
            placeholder="Player ID"
          />
          <button type="button" onClick={() => loadPlayer(playerIdInput)}>Charger</button>
        </div>
        {error ? <p className="player-error">{error}</p> : null}
        {player ? (
          <div className="player-boot-data">
            <p><strong>Username:</strong> {player.username}</p>
            <p><strong>Coins:</strong> {player.money}</p>
            <p><strong>Avatar:</strong> {player.avatar || "(vide)"}</p>
          </div>
        ) : null}
      </div>

      <Link to="/game" className="play-link">
        <img src={playButton} alt="Play" className="play-button"
        />
      </Link>
    </div>
  )
}

export default Home

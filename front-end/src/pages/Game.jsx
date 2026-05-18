import "../styles/Game.css"
import GameBoard from "../components/GameBoard"

const Game = () => {
  return (
    <div className="page-shell">
      <section className="game-area page-card">
        <GameBoard />
      </section>
    </div>
  )
}

export default Game
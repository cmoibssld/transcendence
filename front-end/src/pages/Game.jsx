import "../styles/Game.css"
import Table from "../components/Table"
import Dealer from "../components/Dealer"

const Game = () => {
  return (
    <div className="page-shell">
      <section className="game-area page-card">
          <Dealer />
          <Table />
      </section>
    </div>
  )
}

export default Game
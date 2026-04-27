import "../styles/Game.css"
import Table from "../components/Table"
import Dealer from "../components/Dealer"

const Game = () => {
  return (
    <div className="game-area">
        <Dealer />
        <Table />
    </div>
  )
}

export default Game
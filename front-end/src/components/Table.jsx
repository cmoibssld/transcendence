import tapis from "../assets/background.png"
import "../styles/Table.css"
import { useEffect, useState } from "react"

const GAME_URL = import.meta.env.VITE_GAME_URL || "https://localhost/api/game"

const Table = () => {
  const [tables, setTables] = useState([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState("")

  useEffect(() => {
    let mounted = true
    const fetchTables = async () => {
      try {
        if (mounted) {
          setError("")
        }
        const res = await fetch(`${GAME_URL}/tables`)
        if (!res.ok) {
          throw new Error(`HTTP ${res.status}`)
        }
        const data = await res.json()
        if (mounted) {
          setTables(data)
          setLoading(false)
        }
      } catch (e) {
        if (mounted) {
          setError("Impossible de charger les tables actives.")
          setLoading(false)
        }
      }
    }
    fetchTables()
    const id = setInterval(fetchTables, 3000)
    return () => { mounted = false; clearInterval(id) }
  }, [])

  const activeTables = tables.filter((table) => table.open)

  return (
    <div className="table">
      <div className="table-list">
        <h3>Tables disponibles</h3>
        {loading && <div>Chargement...</div>}
        {error && <div className="table-error">{error}</div>}
        {!loading && !error && tables.length === 0 && <div>Aucune table trouvée.</div>}
        {!loading && !error && activeTables.length > 0 && <div>{activeTables.length} table{activeTables.length > 1 ? "s" : ""} active{activeTables.length > 1 ? "s" : ""}</div>}
        <ul>
          {tables.map((t) => (
            <li key={t.table_id}>
              Table {t.table_id} — joueurs: {t.players} — {t.open ? 'ouverte' : 'fermée'}
              <a href={`/game?table_id=${t.table_id}`}> Rejoindre</a>
            </li>
          ))}
        </ul>
      </div>
      <img src={tapis} alt="tapis" className="table-img" />
    </div>
  )
}

export default Table
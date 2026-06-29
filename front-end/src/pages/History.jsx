import { useState, useEffect } from "react";
import { apiFetch } from "../auth";
import { resolveHttpUrl } from "../lib/publicUrls";
import "../styles/History.css";

const GAME_URL = resolveHttpUrl(import.meta.env.VITE_GAME_URL, "/api/game");

const OUTCOME_META = {
  win:       { label: "Win",       color: "#48c96a", bg: "rgba(72,201,106,0.10)"  },
  blackjack: { label: "Blackjack", color: "#f2c461", bg: "rgba(242,196,97,0.12)"  },
  push:      { label: "Push",      color: "#8888bb", bg: "rgba(136,136,187,0.10)" },
  loss:      { label: "Loss",      color: "#e05a5a", bg: "rgba(224,90,90,0.10)"   },
};

function OutcomeBadge({ outcome }) {
  const m = OUTCOME_META[outcome] ?? OUTCOME_META.loss;
  return (
    <span className="outcome-badge" style={{ color: m.color, background: m.bg }}>
      {m.label}
    </span>
  );
}

function formatDate(iso) {
  const d = new Date(iso);
  return d.toLocaleDateString("fr-FR", { day: "2-digit", month: "short", year: "numeric" })
    + " · " + d.toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" });
}

function fmt(n) { return n?.toLocaleString() ?? "0"; }

export default function History() {
  const [rounds,  setRounds]  = useState([]);
  const [loading, setLoading] = useState(true);
  const [error,   setError]   = useState("");

  useEffect(() => {
    if (!localStorage.getItem('user_id')) {
      setError("Connecte-toi pour voir ton historique.");
      setLoading(false);
      return;
    }
    apiFetch(`${GAME_URL}/history`)
      .then(r => {
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        return r.json();
      })
      .then(data => { setRounds(data); setLoading(false); })
      .catch((e) => { setError(`Could not load history. ${e.message}`); setLoading(false); });
  }, []);

  return (
    <div className="page-shell">
      <section className="page-card history-page">
        <span className="section-chip">Timeline</span>
        <h1 className="page-title" style={{ marginTop: 12 }}>Game History</h1>

        {loading && <p className="history-muted">Loading…</p>}
        {error   && <p className="history-muted error">{error}</p>}

        {!loading && !error && rounds.length === 0 && (
          <div className="history-empty">
            <p>No rounds played yet.</p>
            <span>Play a game to see your history here.</span>
          </div>
        )}

        {rounds.length > 0 && (
          <div className="history-list">
            {rounds.map((r, i) => {
              const profit = r.payout - r.bet;
              return (
                <div className="history-row" key={i}>
                  <div className="history-row-left">
                    <OutcomeBadge outcome={r.outcome} />
                    <span className="history-date">{formatDate(r.played_at)}</span>
                  </div>
                  <div className="history-row-center">
                    <span className="history-detail">Table {r.table_id} · Dealer {r.dealer_points} pts</span>
                  </div>
                  <div className="history-row-right">
                    <span className="history-bet">Bet {fmt(r.bet)} ¢</span>
                    <span
                      className="history-profit"
                      style={{ color: profit > 0 ? "#48c96a" : profit < 0 ? "#e05a5a" : "var(--muted)" }}
                    >
                      {profit >= 0 ? "+" : ""}{fmt(profit)} ¢
                    </span>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </section>
    </div>
  );
}

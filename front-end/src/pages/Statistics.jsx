import { useState, useEffect } from "react";
import { apiFetch, getAccessToken } from "../auth";
import "../styles/Statistics.css";

const GAME_URL = import.meta.env.VITE_GAME_URL ?? "http://localhost:8085";

function Avatar({ username, avatarUrl, size = 32 }) {
  if (avatarUrl)
    return <img className="lb-avatar" src={avatarUrl} alt={username} style={{ width: size, height: size }} />;
  return (
    <div className="lb-avatar-initial" style={{ width: size, height: size }}>
      {username[0].toUpperCase()}
    </div>
  );
}

function LeaderboardTable({ title, rows, valueKey, valueLabel, valueFormat, computeValue }) {
  if (!rows.length) return <p className="stats-muted">No data yet.</p>;
  return (
    <div className="lb-table-wrap">
      <h3 className="lb-title">{title}</h3>
      <table className="lb-table">
        <thead>
          <tr>
            <th>#</th>
            <th>Player</th>
            <th>{valueLabel}</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r, i) => {
            const val = computeValue ? computeValue(r) : r[valueKey];
            return (
              <tr key={r.user_id} className={i < 3 ? `lb-top-${i + 1}` : ""}>
                <td className="lb-rank">{i + 1}</td>
                <td className="lb-player">
                  <Avatar username={r.username} avatarUrl={r.avatar_url} />
                  <span>{r.username}</span>
                </td>
                <td className="lb-value">{valueFormat(val)}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

function StatCard({ label, value, accent }) {
  return (
    <div className="stat-card">
      <span className="stat-card-label">{label}</span>
      <span className="stat-card-value" style={accent ? { color: accent } : {}}>{value}</span>
    </div>
  );
}

export default function Statistics() {
  const [byWins,    setByWins]    = useState([]);
  const [byWagered, setByWagered] = useState([]);
  const [byGains,   setByGains]   = useState([]);
  const [myStats,   setMyStats]   = useState(null);
  const [loading,   setLoading]   = useState(true);

  useEffect(() => {
    const token = getAccessToken();
    Promise.all([
      fetch(`${GAME_URL}/leaderboard/wins?limit=10`).then(r => r.ok ? r.json() : []),
      fetch(`${GAME_URL}/leaderboard/wagered?limit=10`).then(r => r.ok ? r.json() : []),
      fetch(`${GAME_URL}/leaderboard/gains?limit=10`).then(r => r.ok ? r.json() : []),
      token
        ? apiFetch(`${GAME_URL}/stats/me`).then(r => r.ok ? r.json() : null)
        : Promise.resolve(null),
    ]).then(([wins, wagered, gains, me]) => {
      setByWins(wins);
      setByWagered(wagered);
      setByGains(gains);
      setMyStats(me);
      setLoading(false);
    });
  }, []);

  const fmt     = (n) => n?.toLocaleString() ?? "—";
  const fmtCoin = (n) => `${n?.toLocaleString() ?? "—"} ¢`;
  const netGain = (r) => (r.total_won ?? 0) - (r.total_wagered ?? 0);

  return (
    <div className="page-shell">
      <section className="page-card stats-page">
        <span className="section-chip">Overview</span>
        <h1 className="page-title" style={{ marginTop: 12 }}>Statistics</h1>

        {/* ── Personal stats ── */}
        {myStats && (
          <>
            <h3 className="lb-title" style={{ marginTop: 24 }}>Your stats</h3>
            <div className="stat-cards-grid">
              <StatCard label="Games played" value={fmt(myStats.games_played)} />
              <StatCard label="Wins"          value={fmt(myStats.wins)}         accent="#48c96a" />
              <StatCard label="Losses"        value={fmt(myStats.losses)}       accent="#e05a5a" />
              <StatCard label="Pushes"        value={fmt(myStats.pushes)} />
              <StatCard label="Total wagered" value={fmtCoin(myStats.total_wagered)} />
              <StatCard label="Total won"     value={fmtCoin(myStats.total_won)}     accent="#f2c461" />
              <StatCard
                label="Net gain"
                value={fmtCoin((myStats.total_won ?? 0) - (myStats.total_wagered ?? 0))}
                accent={(myStats.total_won ?? 0) >= (myStats.total_wagered ?? 0) ? "#48c96a" : "#e05a5a"}
              />
            </div>
          </>
        )}

        {/* ── Leaderboards ── */}
        {loading ? (
          <p className="stats-muted" style={{ marginTop: 32 }}>Loading leaderboard…</p>
        ) : (
          <>
            <div className="lb-grid" style={{ marginTop: 32 }}>
              <LeaderboardTable
                title="🏆 Top Winners"
                rows={byWins}
                valueKey="wins"
                valueLabel="Wins"
                valueFormat={fmt}
              />
              <LeaderboardTable
                title="💰 Biggest Spenders"
                rows={byWagered}
                valueKey="total_wagered"
                valueLabel="Wagered"
                valueFormat={fmtCoin}
              />
            </div>

            {/* Gains leaderboard — full width, highlighted */}
            {byGains.length > 0 && (
              <div className="lb-gains-section">
                <div className="lb-gains-header">
                  <h3 className="lb-title">📈 Biggest Earners</h3>
                  <span className="lb-gains-sub">Ranked by net profit (total won − total wagered)</span>
                </div>
                <table className="lb-table lb-table--gains">
                  <thead>
                    <tr>
                      <th>#</th>
                      <th>Player</th>
                      <th>Games</th>
                      <th>Wins</th>
                      <th>Net gain</th>
                    </tr>
                  </thead>
                  <tbody>
                    {byGains.map((r, i) => {
                      const gain = netGain(r);
                      return (
                        <tr key={r.user_id} className={i < 3 ? `lb-top-${i + 1}` : ""}>
                          <td className="lb-rank">{i + 1}</td>
                          <td className="lb-player">
                            <Avatar username={r.username} avatarUrl={r.avatar_url} size={28} />
                            <span>{r.username}</span>
                          </td>
                          <td className="lb-secondary">{fmt(r.games_played)}</td>
                          <td className="lb-secondary">{fmt(r.wins)}</td>
                          <td className="lb-value lb-gain-positive">+{fmtCoin(gain)}</td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            )}
          </>
        )}
      </section>
    </div>
  );
}

import { useState, useEffect, useCallback } from "react";
import { apiFetch } from "../auth";
import "../styles/Friends.css";

const AUTH_URL = import.meta.env.VITE_AUTH_URL ?? "http://localhost:8081";
const GAME_URL = import.meta.env.VITE_GAME_URL ?? "http://localhost:8085";

function Avatar({ username, avatarUrl, size = 48 }) {
  if (avatarUrl) {
    return <img className="friend-avatar" src={avatarUrl} alt={username} style={{ width: size, height: size }} />;
  }
  return (
    <div className="friend-avatar-initial" style={{ width: size, height: size }}>{username[0].toUpperCase()}</div>
  );
}

function FriendStatsPanel({ username, onClose }) {
  const [stats, setStats]     = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError]     = useState("");

  useEffect(() => {
    setLoading(true);
    setError("");
    apiFetch(`${GAME_URL}/stats/friend?username=${encodeURIComponent(username)}`)
      .then(r => {
        if (r.ok) return r.json();
        return r.json().catch(() => null).then(body => {
          throw new Error(body?.error ?? (r.status === 403 ? "Not friends." : "Failed to load."));
        });
      })
      .then(data => { setStats(data); setLoading(false); })
      .catch(err => { setError(err.message); setLoading(false); });
  }, [username]);

  const fmt     = (n) => n?.toLocaleString() ?? "—";
  const fmtCoin = (n) => `${n?.toLocaleString() ?? "—"} ¢`;
  const net     = stats ? (stats.total_won ?? 0) - (stats.total_wagered ?? 0) : 0;

  return (
    <div className="friend-stats-panel">
      <div className="friend-stats-header">
        <span className="friend-stats-title">Stats — {username}</span>
        <button className="friend-stats-close" onClick={onClose} aria-label="Close">✕</button>
      </div>

      {loading && <p className="friends-hint">Loading…</p>}
      {error   && <p className="friends-hint error">{error}</p>}

      {stats && (
        <div className="friend-stats-grid">
          <div className="fstat-card">
            <span className="fstat-label">Games played</span>
            <span className="fstat-value">{fmt(stats.games_played)}</span>
          </div>
          <div className="fstat-card">
            <span className="fstat-label">Wins</span>
            <span className="fstat-value" style={{ color: "#48c96a" }}>{fmt(stats.wins)}</span>
          </div>
          <div className="fstat-card">
            <span className="fstat-label">Losses</span>
            <span className="fstat-value" style={{ color: "#e05a5a" }}>{fmt(stats.losses)}</span>
          </div>
          <div className="fstat-card">
            <span className="fstat-label">Pushes</span>
            <span className="fstat-value">{fmt(stats.pushes)}</span>
          </div>
          <div className="fstat-card">
            <span className="fstat-label">Total wagered</span>
            <span className="fstat-value">{fmtCoin(stats.total_wagered)}</span>
          </div>
          <div className="fstat-card">
            <span className="fstat-label">Total won</span>
            <span className="fstat-value" style={{ color: "#f2c461" }}>{fmtCoin(stats.total_won)}</span>
          </div>
          <div className="fstat-card fstat-card--wide">
            <span className="fstat-label">Net gain</span>
            <span className="fstat-value" style={{ color: net >= 0 ? "#48c96a" : "#e05a5a" }}>
              {net >= 0 ? "+" : ""}{fmtCoin(net)}
            </span>
          </div>
        </div>
      )}
    </div>
  );
}

const Friends = () => {
  const [friends, setFriends]         = useState([]);
  const [pending, setPending]         = useState([]);
  const [search, setSearch]           = useState("");
  const [found, setFound]             = useState(null);
  const [searching, setSearching]     = useState(false);
  const [searchErr, setSearchErr]     = useState("");
  const [loading, setLoading]         = useState(true);
  const [viewingStats, setViewingStats] = useState(null);

  const loadFriends = useCallback(async () => {
    const [fRes, pRes] = await Promise.all([
      apiFetch(`${AUTH_URL}/auth/friends`),
      apiFetch(`${AUTH_URL}/auth/friends/pending`),
    ]);
    if (fRes.ok) setFriends(await fRes.json());
    if (pRes.ok) setPending(await pRes.json());
    setLoading(false);
  }, []);

  useEffect(() => { loadFriends(); }, [loadFriends]);

  const handleSearch = async (e) => {
    e.preventDefault();
    const q = search.trim();
    if (!q) return;
    setSearching(true);
    setFound(null);
    setSearchErr("");
    const res = await apiFetch(`${AUTH_URL}/auth/users?username=${encodeURIComponent(q)}`);
    setSearching(false);
    if (res.ok) {
      setFound(await res.json());
    } else if (res.status === 404) {
      setSearchErr("No user found.");
    } else {
      setSearchErr("Search failed.");
    }
  };

  const sendRequest = async (username) => {
    const res = await apiFetch(
      `${AUTH_URL}/auth/friends/request?username=${encodeURIComponent(username)}`,
      { method: "POST" }
    );
    if (res.ok) {
      setFound(null);
      setSearch("");
    } else {
      const body = await res.json().catch(() => ({}));
      setSearchErr(body.error ?? "Could not send request.");
    }
  };

  const accept = async (username) => {
    await apiFetch(
      `${AUTH_URL}/auth/friends/accept?username=${encodeURIComponent(username)}`,
      { method: "POST" }
    );
    loadFriends();
  };

  const decline = async (username) => {
    await apiFetch(
      `${AUTH_URL}/auth/friends/decline?username=${encodeURIComponent(username)}`,
      { method: "POST" }
    );
    if (viewingStats === username) setViewingStats(null);
    loadFriends();
  };

  const isFriend  = (u) => friends.some((f) => f.username === u);
  const isPending = (u) => pending.some((p) => p.username === u);

  return (
    <div className="page-shell">
      <section className="friends-page page-card">

        {/* ── Top bar ────────────────────────────────── */}
        <div className="top-bar">
          <div>
            <span className="section-chip">Community</span>
            <h2 className="friends-label">Friends</h2>
          </div>

          <form className="search-bar" onSubmit={handleSearch}>
            <input
              type="text"
              placeholder="Search by username…"
              value={search}
              onChange={(e) => { setSearch(e.target.value); setFound(null); setSearchErr(""); }}
            />
          </form>
        </div>

        {/* ── Search result ──────────────────────────── */}
        {searching && <p className="friends-hint">Searching…</p>}
        {searchErr  && <p className="friends-hint error">{searchErr}</p>}
        {found && (
          <div className="search-result">
            <Avatar username={found.username} avatarUrl={found.avatar_url} />
            <span className="friend-name">{found.username}</span>
            {isFriend(found.username) ? (
              <span className="badge">Already friends</span>
            ) : isPending(found.username) ? (
              <div className="action-row">
                <button className="btn-accept" onClick={() => accept(found.username)}>Accept</button>
                <button className="btn-decline" onClick={() => decline(found.username)}>Decline</button>
              </div>
            ) : (
              <button className="btn-add" onClick={() => sendRequest(found.username)}>Add friend</button>
            )}
          </div>
        )}

        {/* ── Pending requests ───────────────────────── */}
        {pending.length > 0 && (
          <>
            <h3 className="subsection-title">Pending requests</h3>
            <div className="friends-grid">
              {pending.map((p) => (
                <div className="friend-card" key={p.id}>
                  <Avatar username={p.username} avatarUrl={p.avatar_url} />
                  <span className="friend-name">{p.username}</span>
                  <div className="action-row">
                    <button className="btn-accept"  onClick={() => accept(p.username)}>Accept</button>
                    <button className="btn-decline" onClick={() => decline(p.username)}>Decline</button>
                  </div>
                </div>
              ))}
            </div>
          </>
        )}

        {/* ── Friends list ───────────────────────────── */}
        <h3 className="subsection-title">My friends</h3>
        {loading ? (
          <p className="friends-hint">Loading…</p>
        ) : friends.length > 0 ? (
          <>
            <div className="friends-grid">
              {friends.map((f) => (
                <div
                  className={`friend-card${viewingStats === f.username ? " friend-card--active" : ""}`}
                  key={f.id}
                >
                  <Avatar username={f.username} avatarUrl={f.avatar_url} />
                  <span className="friend-name">{f.username}</span>
                  <div className="action-row">
                    <button
                      className="btn-stats"
                      onClick={() => setViewingStats(v => v === f.username ? null : f.username)}
                    >
                      {viewingStats === f.username ? "Hide" : "Stats"}
                    </button>
                    <button className="btn-decline" onClick={() => decline(f.username)}>Remove</button>
                  </div>
                </div>
              ))}
            </div>

            {viewingStats && (
              <FriendStatsPanel
                username={viewingStats}
                onClose={() => setViewingStats(null)}
              />
            )}
          </>
        ) : (
          <div className="no-friends">
            <p>No friends yet.</p>
            <span>Search by username to add your first friend.</span>
          </div>
        )}

      </section>
    </div>
  );
};

export default Friends;

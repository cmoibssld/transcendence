import { useEffect, useState } from 'react'
import '../styles/Profile.css'

const AUTH_URL = import.meta.env.VITE_AUTH_URL ?? 'https://localhost:8443'

function AvatarDisplay({ url, username }) {
  if (url) {
    return <img src={url} alt={username} className="profile-avatar-img" />
  }
  const initial = (username ?? 'J')[0].toUpperCase()
  return <div className="profile-avatar-initials">{initial}</div>
}

export default function Profile() {
  const [data,    setData]    = useState(null)
  const [loading, setLoading] = useState(true)
  const [error,   setError]   = useState('')

  useEffect(() => {
    const token = localStorage.getItem('access_token')
    if (!token) { setError('Non authentifié'); setLoading(false); return }

    fetch(`${AUTH_URL}/auth/profile`, {
      headers: { Authorization: `Bearer ${token}` },
    })
      .then(r => r.ok ? r.json() : Promise.reject(r.status))
      .then(d => { setData(d); setLoading(false) })
      .catch(() => { setError('Impossible de charger le profil'); setLoading(false) })
  }, [])

  if (loading) {
    return (
      <div className="page-shell">
        <section className="page-card profile-page">
          <p className="profile-loading">Chargement…</p>
        </section>
      </div>
    )
  }

  if (error) {
    return (
      <div className="page-shell">
        <section className="page-card profile-page">
          <p className="profile-error">{error}</p>
        </section>
      </div>
    )
  }

  const isGitHub = !!data.avatar_url

  return (
    <div className="page-shell">
      <section className="page-card profile-page">
        <div className="profile-inner">

          <div className="profile-sidebar">
            <span className="section-chip">Profil</span>
            <div className="profile-avatar">
              <AvatarDisplay url={data.avatar_url} username={data.username} />
            </div>
            <h2 className="profile-name">{data.username}</h2>
            <p className="profile-account-type">
              {isGitHub ? 'Compte GitHub' : 'Compte local'}
            </p>
          </div>

          <div className="profile-stats">
            <div className="profile-stat-card">
              <span className="profile-stat-label">Coins</span>
              <strong className="profile-stat-value">{data.coins.toLocaleString()}</strong>
            </div>
            <div className="profile-stat-card">
              <span className="profile-stat-label">Pseudo</span>
              <strong className="profile-stat-value">{data.username}</strong>
            </div>
          </div>

        </div>
      </section>
    </div>
  )
}

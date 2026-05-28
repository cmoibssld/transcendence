import { useEffect, useState, useCallback } from 'react'
import '../styles/Profile.css'
import { apiFetch } from '../auth'

const AUTH_URL = import.meta.env.VITE_AUTH_URL ?? 'https://localhost:8443'

function AvatarDisplay({ url, username }) {
  if (url) {
    return <img src={url} alt={username} className="profile-avatar-img" />
  }
  return (
    <div className="profile-avatar-initials">
      {(username ?? 'J')[0].toUpperCase()}
    </div>
  )
}

export default function Profile() {
  const [data,        setData]        = useState(null)
  const [loading,     setLoading]     = useState(true)
  const [error,       setError]       = useState('')
  const [editing,     setEditing]     = useState(false)
  const [saving,      setSaving]      = useState(false)
  const [saveError,   setSaveError]   = useState('')
  const [saveSuccess, setSaveSuccess] = useState('')
  const [form, setForm] = useState({
    username:         '',
    avatar_url:       '',
    bio:              '',
    current_password: '',
    new_password:     '',
  })

  const loadProfile = useCallback(() => {
    setLoading(true)
    apiFetch(`${AUTH_URL}/auth/profile`)
      .then(r => r.ok ? r.json() : Promise.reject(r.status))
      .then(d => {
        setData(d)
        setForm(f => ({
          ...f,
          username:   d.username,
          avatar_url: d.avatar_url ?? '',
          bio:        d.bio ?? '',
        }))
        setLoading(false)
      })
      .catch(() => { setError('Impossible de charger le profil'); setLoading(false) })
  }, [])

  useEffect(() => { loadProfile() }, [loadProfile])

  const startEdit = () => {
    setSaveError('')
    setSaveSuccess('')
    setForm(f => ({ ...f, current_password: '', new_password: '' }))
    setEditing(true)
  }

  const cancelEdit = () => {
    setEditing(false)
    setSaveError('')
    if (data) {
      setForm(f => ({
        ...f,
        username:         data.username,
        avatar_url:       data.avatar_url ?? '',
        bio:              data.bio ?? '',
        current_password: '',
        new_password:     '',
      }))
    }
  }

  const handleSave = async (e) => {
    e.preventDefault()
    setSaving(true)
    setSaveError('')
    setSaveSuccess('')

    const body = {}
    if (form.username !== data.username) body.username = form.username.trim()
    if (form.avatar_url !== (data.avatar_url ?? '')) body.avatar_url = form.avatar_url
    if (form.bio !== (data.bio ?? '')) body.bio = form.bio
    if (form.new_password) {
      body.current_password = form.current_password
      body.new_password = form.new_password
    }

    if (Object.keys(body).length === 0) {
      setEditing(false)
      setSaving(false)
      return
    }

    try {
      const res  = await apiFetch(`${AUTH_URL}/auth/profile`, {
        method:  'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body:    JSON.stringify(body),
      })
      const json = await res.json()
      if (!res.ok) throw new Error(json.error ?? 'Échec de la mise à jour')

      setData(json)
      setForm(f => ({
        ...f,
        username:         json.username,
        avatar_url:       json.avatar_url ?? '',
        bio:              json.bio ?? '',
        current_password: '',
        new_password:     '',
      }))
      localStorage.setItem('username',   json.username)
      localStorage.setItem('avatar_url', json.avatar_url ?? '')
      setEditing(false)
      setSaveSuccess('Profil mis à jour avec succès.')
    } catch (err) {
      setSaveError(err.message)
    } finally {
      setSaving(false)
    }
  }

  const change = (field) => (e) => setForm(f => ({ ...f, [field]: e.target.value }))

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

  const isLocal       = data.provider === 'local'
  const providerLabel = data.provider === 'github' ? '— GitHub' : '— Local'

  return (
    <div className="page-shell">
      <section className="page-card profile-page">
        <div className="profile-inner">

          {/* ── Sidebar ── */}
          <aside className="profile-sidebar">
            <div className="profile-avatar">
              <AvatarDisplay url={data.avatar_url} username={data.username} />
            </div>

            <div className="profile-sidebar-info">
              <h2 className="profile-name">{data.username}</h2>
              <p className="profile-account-type">{providerLabel}</p>
            </div>

            <div className="profile-coins-pill">
              <span className="profile-coins-icon">¢</span>
              <strong>{data.coins.toLocaleString()}</strong>
              <span className="profile-coins-label">coins</span>
            </div>

            {!editing ? (
              <button className="profile-edit-btn" onClick={startEdit}>
                Modifier le profil
              </button>
            ) : (
              <button className="profile-cancel-btn" onClick={cancelEdit} disabled={saving}>
                Annuler
              </button>
            )}
          </aside>

          {/* ── Main content ── */}
          <div className="profile-main">

            {!editing ? (
              <>
                {saveSuccess && (
                  <div className="profile-alert profile-alert--success">{saveSuccess}</div>
                )}

                <div className="profile-view-grid">
                  <div className="profile-view-card">
                    <span className="profile-view-label">Pseudo</span>
                    <span className="profile-view-value">{data.username}</span>
                  </div>

                  <div className="profile-view-card">
                    <span className="profile-view-label">Solde</span>
                    <span className="profile-view-value profile-view-value--accent">
                      {data.coins.toLocaleString()} ¢
                    </span>
                  </div>

                  <div className="profile-view-card profile-view-card--wide">
                    <span className="profile-view-label">Bio</span>
                    {data.bio
                      ? <span className="profile-view-value">{data.bio}</span>
                      : <em className="profile-view-empty">Aucune bio — cliquez sur Modifier pour en ajouter une.</em>
                    }
                  </div>

                  {data.avatar_url && (
                    <div className="profile-view-card profile-view-card--wide">
                      <span className="profile-view-label">Avatar URL</span>
                      <span className="profile-view-value profile-view-value--url">{data.avatar_url}</span>
                    </div>
                  )}
                </div>
              </>
            ) : (
              <form className="profile-form" onSubmit={handleSave}>
                <h3 className="profile-form-title">Modifier le profil</h3>

                <div className="profile-field">
                  <label htmlFor="pf-username">Pseudo</label>
                  <input
                    id="pf-username"
                    type="text"
                    value={form.username}
                    onChange={change('username')}
                    maxLength={50}
                    required
                  />
                </div>

                <div className="profile-field">
                  <label htmlFor="pf-bio">
                    Bio
                    <span className="profile-field-counter">
                      {form.bio.length}/300
                    </span>
                  </label>
                  <textarea
                    id="pf-bio"
                    value={form.bio}
                    onChange={change('bio')}
                    maxLength={300}
                    rows={3}
                    placeholder="Décrivez-vous en quelques mots…"
                  />
                </div>

                <div className="profile-field">
                  <label htmlFor="pf-avatar">URL de l'avatar</label>
                  <input
                    id="pf-avatar"
                    type="url"
                    value={form.avatar_url}
                    onChange={change('avatar_url')}
                    placeholder="https://…"
                  />
                  {form.avatar_url && (
                    <div className="profile-avatar-preview">
                      <img
                        src={form.avatar_url}
                        alt="Aperçu"
                        onError={e => { e.target.style.display = 'none' }}
                      />
                    </div>
                  )}
                </div>

                {isLocal && (
                  <div className="profile-password-section">
                    <p className="profile-password-title">
                      <span className="profile-password-icon">🔒</span>
                      Changer le mot de passe
                    </p>
                    <div className="profile-field">
                      <label htmlFor="pf-cur-pass">Mot de passe actuel</label>
                      <input
                        id="pf-cur-pass"
                        type="password"
                        value={form.current_password}
                        onChange={change('current_password')}
                        autoComplete="current-password"
                        placeholder="••••••••"
                      />
                    </div>
                    <div className="profile-field">
                      <label htmlFor="pf-new-pass">Nouveau mot de passe</label>
                      <input
                        id="pf-new-pass"
                        type="password"
                        value={form.new_password}
                        onChange={change('new_password')}
                        autoComplete="new-password"
                        placeholder="••••••••"
                      />
                    </div>
                  </div>
                )}

                {saveError && (
                  <p className="profile-save-error" role="alert">{saveError}</p>
                )}

                <div className="profile-form-actions">
                  <button type="submit" className="profile-save-btn" disabled={saving}>
                    {saving ? 'Enregistrement…' : 'Enregistrer'}
                  </button>
                  <button
                    type="button"
                    className="profile-cancel-btn-inline"
                    onClick={cancelEdit}
                    disabled={saving}
                  >
                    Annuler
                  </button>
                </div>
              </form>
            )}
          </div>

        </div>
      </section>
    </div>
  )
}

import { useState, useEffect } from 'react'
import { apiFetch } from '../auth'

const AUTH_URL = import.meta.env.VITE_AUTH_URL ?? 'https://localhost:8443'

export default function Settings() {
  const [profile,  setProfile]  = useState(null)
  const [loading,  setLoading]  = useState(true)
  const [form,     setForm]     = useState({ current_password: '', new_password: '', confirm_password: '' })
  const [saving,   setSaving]   = useState(false)
  const [error,    setError]    = useState('')
  const [success,  setSuccess]  = useState('')

  useEffect(() => {
    apiFetch(`${AUTH_URL}/auth/profile`)
      .then(r => r.ok ? r.json() : Promise.reject())
      .then(d => { setProfile(d); setLoading(false) })
      .catch(() => setLoading(false))
  }, [])

  const change = (field) => (e) => {
    setError('')
    setSuccess('')
    setForm(f => ({ ...f, [field]: e.target.value }))
  }

  const handlePasswordChange = async (e) => {
    e.preventDefault()
    if (form.new_password !== form.confirm_password) {
      setError('Les nouveaux mots de passe ne correspondent pas.')
      return
    }
    if (form.new_password.length < 8) {
      setError('Le nouveau mot de passe doit contenir au moins 8 caractères.')
      return
    }
    setSaving(true)
    setError('')
    setSuccess('')
    try {
      const res = await apiFetch(`${AUTH_URL}/auth/profile`, {
        method:  'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body:    JSON.stringify({
          current_password: form.current_password,
          new_password:     form.new_password,
        }),
      })
      const json = await res.json()
      if (!res.ok) throw new Error(json.error ?? 'Erreur lors du changement de mot de passe')
      setSuccess('Mot de passe mis à jour avec succès.')
      setForm({ current_password: '', new_password: '', confirm_password: '' })
    } catch (err) {
      setError(err.message)
    } finally {
      setSaving(false)
    }
  }

  if (loading) {
    return (
      <div className="page-shell">
        <section className="page-card" style={{ padding: 28 }}>
          <p className="page-copy">Chargement…</p>
        </section>
      </div>
    )
  }

  const isLocal = profile?.provider === 'local'

  return (
    <div className="page-shell">
      <section className="page-card" style={{ padding: 28, maxWidth: 520 }}>
        <span className="section-chip">Compte</span>
        <h1 className="page-title" style={{ marginTop: 12, marginBottom: 20 }}>Settings</h1>

        {profile && (
          <div style={{ marginBottom: 28, padding: '12px 16px', background: 'rgba(255,255,255,0.04)', borderRadius: 8 }}>
            <p style={{ margin: 0, fontSize: 14, color: 'var(--text-muted, #aaa)' }}>Connecté en tant que</p>
            <p style={{ margin: '4px 0 0', fontWeight: 600 }}>{profile.username}</p>
            <p style={{ margin: '2px 0 0', fontSize: 13, color: 'var(--text-muted, #aaa)', textTransform: 'capitalize' }}>
              Compte {profile.provider === 'github' ? 'GitHub OAuth' : 'local'}
            </p>
          </div>
        )}

        {isLocal ? (
          <>
            <h2 style={{ fontSize: 16, marginBottom: 16 }}>Changer le mot de passe</h2>
            <form onSubmit={handlePasswordChange}>
              <div className="profile-field" style={{ marginBottom: 14 }}>
                <label htmlFor="s-cur-pass" style={{ display: 'block', marginBottom: 4, fontSize: 13 }}>
                  Mot de passe actuel
                </label>
                <input
                  id="s-cur-pass"
                  type="password"
                  value={form.current_password}
                  onChange={change('current_password')}
                  autoComplete="current-password"
                  placeholder="••••••••"
                  required
                  style={{ width: '100%', padding: '8px 10px', borderRadius: 6, border: '1px solid rgba(255,255,255,0.15)', background: 'rgba(255,255,255,0.06)', color: 'inherit', fontSize: 14 }}
                />
              </div>

              <div className="profile-field" style={{ marginBottom: 14 }}>
                <label htmlFor="s-new-pass" style={{ display: 'block', marginBottom: 4, fontSize: 13 }}>
                  Nouveau mot de passe
                </label>
                <input
                  id="s-new-pass"
                  type="password"
                  value={form.new_password}
                  onChange={change('new_password')}
                  autoComplete="new-password"
                  placeholder="••••••••"
                  minLength={8}
                  required
                  style={{ width: '100%', padding: '8px 10px', borderRadius: 6, border: '1px solid rgba(255,255,255,0.15)', background: 'rgba(255,255,255,0.06)', color: 'inherit', fontSize: 14 }}
                />
              </div>

              <div className="profile-field" style={{ marginBottom: 20 }}>
                <label htmlFor="s-confirm-pass" style={{ display: 'block', marginBottom: 4, fontSize: 13 }}>
                  Confirmer le nouveau mot de passe
                </label>
                <input
                  id="s-confirm-pass"
                  type="password"
                  value={form.confirm_password}
                  onChange={change('confirm_password')}
                  autoComplete="new-password"
                  placeholder="••••••••"
                  minLength={8}
                  required
                  style={{ width: '100%', padding: '8px 10px', borderRadius: 6, border: '1px solid rgba(255,255,255,0.15)', background: 'rgba(255,255,255,0.06)', color: 'inherit', fontSize: 14 }}
                />
              </div>

              {error   && <p role="alert" style={{ color: '#e57373', marginBottom: 12, fontSize: 13 }}>{error}</p>}
              {success && <p role="status" style={{ color: '#81c784', marginBottom: 12, fontSize: 13 }}>{success}</p>}

              <button
                type="submit"
                disabled={saving}
                style={{ padding: '9px 20px', borderRadius: 6, border: 'none', background: 'var(--accent, #c4a882)', color: '#1a1a22', fontWeight: 600, cursor: saving ? 'not-allowed' : 'pointer', opacity: saving ? 0.7 : 1 }}
              >
                {saving ? 'Enregistrement…' : 'Mettre à jour le mot de passe'}
              </button>
            </form>
          </>
        ) : (
          <div style={{ padding: '14px 16px', background: 'rgba(255,255,255,0.04)', borderRadius: 8, fontSize: 14, color: 'var(--text-muted, #aaa)' }}>
            Le mot de passe est géré par votre compte GitHub OAuth et ne peut pas être modifié ici.
          </div>
        )}

        <div style={{ marginTop: 28, paddingTop: 20, borderTop: '1px solid rgba(255,255,255,0.08)' }}>
          <p style={{ fontSize: 13, color: 'var(--text-muted, #aaa)', margin: 0 }}>
            Pour modifier votre pseudo, bio ou avatar, rendez-vous sur la page{' '}
            <a href="/profile" style={{ color: 'var(--accent, #c4a882)' }}>Profil</a>.
          </p>
        </div>
      </section>
    </div>
  )
}

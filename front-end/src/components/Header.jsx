import { useNavigate, Link, NavLink } from 'react-router-dom'
import { useState, useEffect } from 'react'
import { logout as authLogout } from '../auth'
import '../styles/Header.css'

const NAV_PUBLIC  = [
  { to: '/',      label: 'Accueil' },
  { to: '/rules', label: 'Règles'  },
]

const NAV_AUTH = [
  { to: '/',           label: 'Accueil'    },
  { to: '/friends',    label: 'Amis'       },
  { to: '/statistics', label: 'Stats'      },
  { to: '/history',    label: 'Historique' },
  { to: '/rules',      label: 'Règles'     },
]

function Avatar({ url, username }) {
  if (url) {
    return <img src={url} alt={username} className="hdr-avatar-img" />
  }
  const initial = (username ?? 'J')[0].toUpperCase()
  return <span className="hdr-avatar-initials">{initial}</span>
}

function LogoutIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/>
      <polyline points="16 17 21 12 16 7"/>
      <line x1="21" y1="12" x2="9" y2="12"/>
    </svg>
  )
}

export default function Header() {
  const navigate   = useNavigate()
  const isLoggedIn = !!localStorage.getItem('user_id')
  const [username,  setUsername]  = useState(localStorage.getItem('username') ?? 'Joueur')
  const [avatarUrl, setAvatarUrl] = useState(localStorage.getItem('avatar_url') || null)

  useEffect(() => {
    const sync = () => {
      setUsername(localStorage.getItem('username') ?? 'Joueur')
      setAvatarUrl(localStorage.getItem('avatar_url') || null)
    }
    window.addEventListener('profile-updated', sync)
    window.addEventListener('storage', sync)
    return () => {
      window.removeEventListener('profile-updated', sync)
      window.removeEventListener('storage', sync)
    }
  }, [])

  const navLinks = isLoggedIn ? NAV_AUTH : NAV_PUBLIC

  function handleLogout() {
    authLogout()  // clears cookies server-side then clears localStorage and redirects
  }

  return (
    <header className="hdr">
      <Link to="/" className="hdr-brand">
        <span className="hdr-brand__suit" aria-hidden="true">♠</span>
        <span className="hdr-brand__text">LUIGI JACK</span>
      </Link>

      <nav className="hdr-nav">
        {navLinks.map(({ to, label }) => (
          <NavLink
            key={to}
            to={to}
            end={to === '/'}
            className={({ isActive }) => isActive ? 'hdr-link hdr-link--active' : 'hdr-link'}
          >
            {label}
          </NavLink>
        ))}
      </nav>

      <div className="hdr-right">
        {isLoggedIn ? (
          <>
            <Link to="/profile" className="hdr-user">
              <span className="hdr-avatar">
                <Avatar url={avatarUrl} username={username} />
              </span>
              <span className="hdr-username">{username}</span>
            </Link>
            <button
              className="hdr-logout"
              onClick={handleLogout}
              title="Se déconnecter"
              aria-label="Se déconnecter"
            >
              <LogoutIcon />
            </button>
          </>
        ) : (
          <>
            <Link to="/login"    className="hdr-auth-link">Connexion</Link>
            <Link to="/register" className="btn-primary hdr-register">S'inscrire</Link>
          </>
        )}
      </div>
    </header>
  )
}

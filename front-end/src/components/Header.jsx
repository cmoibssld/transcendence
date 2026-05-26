import { useNavigate, Link, NavLink } from 'react-router-dom'
import '../styles/Header.css'

const NAV_PUBLIC  = [
  { to: '/',     label: 'Home'    },
  { to: '/rules', label: 'Règles' },
  { to: '/chat',  label: 'Chat'   },
]

const NAV_AUTH = [
  { to: '/',           label: 'Home'       },
  { to: '/friends',    label: 'Amis'       },
  { to: '/statistics', label: 'Stats'      },
  { to: '/history',    label: 'Historique' },
  { to: '/chat',       label: 'Chat'       },
  { to: '/rules',      label: 'Règles'     },
]

function Avatar({ url, username }) {
  if (url) {
    return <img src={url} alt={username} className="hdr-avatar-img" />
  }
  const initial = (username ?? 'J')[0].toUpperCase()
  return <span className="hdr-avatar-initials">{initial}</span>
}

export default function Header() {
  const navigate  = useNavigate()
  const token     = localStorage.getItem('access_token')
  const isLoggedIn = !!token
  const username  = localStorage.getItem('username') ?? 'Joueur'
  const avatarUrl = localStorage.getItem('avatar_url') || null

  const navLinks = isLoggedIn ? NAV_AUTH : NAV_PUBLIC

  function handleLogout() {
    localStorage.clear()
    navigate('/login')
  }

  return (
    <header className="hdr">
      <Link to="/" className="hdr-brand">LUIGI JACK</Link>

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
            <button className="hdr-logout" onClick={handleLogout} aria-label="Déconnexion">
              ⎋
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

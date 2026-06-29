import { useState } from "react"
import { Link, useLocation } from "react-router-dom"
import { resolveHttpUrl } from "../lib/publicUrls"
import "../styles/Login.css"

const Login = () => {
    const location = useLocation()
    const params = new URLSearchParams(location.search)
    const oauthError = params.get("error") === "oauth_failed"
    const oauthReason = (params.get("reason") ?? "").slice(0, 100).replace(/[<>"']/g, "")

    const [form, setForm] = useState({ username: "", password: "" })
    const [error, setError] = useState("")
    const [loading, setLoading] = useState(false)

    const authBaseUrl = resolveHttpUrl(import.meta.env.VITE_AUTH_URL, "")

    const handleChange = (e) => {
        const { name, value } = e.target
        setForm(cur => ({ ...cur, [name]: value }))
    }

    const handleSubmit = async (e) => {
        e.preventDefault()
        setError("")
        setLoading(true)
        try {
            const res  = await fetch(`${authBaseUrl}/auth/login`, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify(form),
            })
            const data = await res.json()
            if (!res.ok) throw new Error(data?.error ?? "Connexion refusée")
            // Tokens are now in HttpOnly cookies set by the server — store only display info
            if (data.username)  localStorage.setItem("username",  data.username)
            if (data.avatar_url) localStorage.setItem("avatar_url", data.avatar_url ?? "")
            if (data.user_id)   localStorage.setItem("user_id",   String(data.user_id))
            window.location.href = "/"
        } catch (err) {
            setError(err.message)
        } finally {
            setLoading(false)
        }
    }

    const handleGitHubLogin = () => {
        window.location.href = `${authBaseUrl}/auth/github/login`
    }

    return (
        <div className="login-page">
            <div className="login-card">

                {/* ── Casino header ── */}
                <div className="login-portal-header" aria-hidden="true">
                    <div className="login-suits">
                        <span className="login-suit login-suit--black">♠</span>
                        <span className="login-suit login-suit--red">♥</span>
                        <span className="login-suit login-suit--red">♦</span>
                        <span className="login-suit login-suit--black">♣</span>
                    </div>
                </div>

                {/* ── Heading ── */}
                <div className="login-heading">
                    <span className="login-eyebrow">Casino Luigi Jack</span>
                    <h2>Connexion</h2>
                    <p className="login-subtitle">Rejoignez la table.</p>
                </div>

                {/* ── Form ── */}
                <form className="login-form" onSubmit={handleSubmit}>
                    <div className="login-field">
                        <label htmlFor="username">Nom d'utilisateur</label>
                        <input
                            id="username"
                            name="username"
                            type="text"
                            placeholder="username"
                            autoComplete="username"
                            value={form.username}
                            onChange={handleChange}
                        />
                    </div>

                    <div className="login-field">
                        <label htmlFor="password">Mot de passe</label>
                        <input
                            id="password"
                            name="password"
                            type="password"
                            placeholder="••••••••"
                            autoComplete="current-password"
                            value={form.password}
                            onChange={handleChange}
                        />
                    </div>

                    {oauthError && (
                      <p className="login-error" role="alert">
                        Connexion GitHub échouée.{oauthReason ? ` (${oauthReason})` : ""}
                      </p>
                    )}
                    {error && <p className="login-error" role="alert">{error}</p>}

                    <button className="login-btn-primary" type="submit" disabled={loading}>
                        {loading ? "Connexion…" : "Se connecter"}
                    </button>

                    <div className="login-divider"><span>ou</span></div>

                    <button className="login-btn-social" type="button" onClick={handleGitHubLogin}>
                        <GithubIcon />
                        Continuer avec GitHub
                    </button>

                    <Link className="login-btn-social" to="/register">
                        Pas de compte ? <strong>S'inscrire</strong>
                    </Link>
                </form>
            </div>
        </div>
    )
}

function GithubIcon() {
    return (
        <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
            <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0 0 24 12c0-6.63-5.37-12-12-12z"/>
        </svg>
    )
}

export default Login

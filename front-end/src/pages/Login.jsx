import { useState } from "react"
import { Link } from "react-router-dom"
import "../styles/Login.css"

const Login = () => {
    const [form, setForm] = useState({
        username: "",
        password: "",
    })
    const [error, setError] = useState("")
    const [loading, setLoading] = useState(false)

    const authBaseUrl = import.meta.env.VITE_AUTH_URL ?? "http://localhost:8081"

    const handleChange = (event) => {
        const { name, value } = event.target

        setForm((current) => ({
            ...current,
            [name]: value,
        }))
    }

    const handleSubmit = async (event) => {
        event.preventDefault()
        setError("")
        setLoading(true)

        try {
            const response = await fetch(`${authBaseUrl}/auth/login`, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify(form),
            })

            const data = await response.json()

            if (!response.ok) {
                throw new Error(data?.error ?? "Connexion refusee")
            }

            localStorage.setItem("access_token", data.access_token)
            localStorage.setItem("refresh_token", data.refresh_token)
            localStorage.setItem("username", data.username)

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
            <div className="login-glow" />

            <div className="login-card">
                <div className="login-heading">
                    <span className="login-kicker">LUIGI JACK</span>
                    <h2>Connexion</h2>
                    <p className="login-subtitle">Rejoindre le casino.</p>
                </div>

                <form className="login-form" onSubmit={handleSubmit}>
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

                    {error && <p className="login-error">{error}</p>}

                    <button className="primary-button" type="submit" disabled={loading}>
                        {loading ? "Connexion..." : "Se connecter"}
                    </button>

                    <div className="divider">
                        <span>ou</span>
                    </div>

                    <button className="github-button" type="button" onClick={handleGitHubLogin}>
                        Continuer avec GitHub
                    </button>

                    <Link className="github-button register-link" to="/register">
                        Pas de compte ? S'inscrire
                    </Link>
                </form>
            </div>
        </div>
    )
}

export default Login
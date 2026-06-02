import { useState } from "react"
import { Link } from "react-router-dom"
import "../styles/Login.css"

const Register = () => {
    const [form, setForm] = useState({
        username: "",
        email: "",
        password: "",
        confirmPassword: "",
    })
    const [error, setError] = useState("")
    const [success, setSuccess] = useState("")
    const [loading, setLoading] = useState(false)

    const authBaseUrl = import.meta.env.VITE_AUTH_URL ?? "https://localhost:8443"

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
        setSuccess("")

        if (!form.username || !form.email || !form.password) {
            setError("Tous les champs sont obligatoires")
            return
        }

        if (form.username.trim().length < 2 || form.username.length > 32) {
            setError("Le nom d'utilisateur doit contenir entre 2 et 32 caractères")
            return
        }

        if (!/^[a-zA-Z0-9_-]+$/.test(form.username)) {
            setError("Le nom d'utilisateur ne peut contenir que des lettres, chiffres, _ et -")
            return
        }

        if (form.password.length < 8) {
            setError("Le mot de passe doit contenir au moins 8 caractères")
            return
        }

        if (form.password.length > 128) {
            setError("Le mot de passe est trop long (128 caractères max)")
            return
        }

        if (form.password !== form.confirmPassword) {
            setError("Les mots de passe ne correspondent pas")
            return
        }

        const emailTrimmed = form.email.trim()
        if (!emailTrimmed.includes("@") || emailTrimmed.startsWith("@") || emailTrimmed.endsWith("@")) {
            setError("Adresse e-mail invalide")
            return
        }

        setLoading(true)

        try {
            const response = await fetch(`${authBaseUrl}/auth/register`, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify({
                    username: form.username,
                    email: form.email,
                    password: form.password,
                }),
            })

            const data = await response.json()

            if (!response.ok) {
                throw new Error(data?.error ?? "Inscription refusee")
            }

            setSuccess("Compte cree avec succes. Tu peux maintenant te connecter.")
            setForm({ username: "", email: "", password: "", confirmPassword: "" })
        } catch (err) {
            setError(err.message)
        } finally {
            setLoading(false)
        }
    }

    return (
        <div className="login-page">
            <div className="login-card">
                <div className="login-heading">
                    <h2>Inscription</h2>
                    <p className="login-subtitle">Creer un compte joueur.</p>
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

                    <label htmlFor="email">Email</label>
                    <input
                        id="email"
                        name="email"
                        type="email"
                        placeholder="you@example.com"
                        autoComplete="email"
                        value={form.email}
                        onChange={handleChange}
                    />

                    <label htmlFor="password">Mot de passe</label>
                    <input
                        id="password"
                        name="password"
                        type="password"
                        placeholder="••••••••"
                        autoComplete="new-password"
                        value={form.password}
                        onChange={handleChange}
                    />

                    <label htmlFor="confirmPassword">Confirmer le mot de passe</label>
                    <input
                        id="confirmPassword"
                        name="confirmPassword"
                        type="password"
                        placeholder="••••••••"
                        autoComplete="new-password"
                        value={form.confirmPassword}
                        onChange={handleChange}
                    />

                    {error && <p className="login-error text-sm">{error}</p>}
                    {success && <p className="login-success text-sm">{success}</p>}

                    <button className="primary-button" type="submit" disabled={loading}>
                        {loading ? "Creation..." : "Creer un compte"}
                    </button>

                    <div className="divider">
                        <span>ou</span>
                    </div>

                    <Link className="github-button register-link" to="/login">
                        Deja inscrit ? Se connecter
                    </Link>
                </form>
            </div>
        </div>
    )
}

export default Register

const AUTH_URL = import.meta.env.VITE_AUTH_URL ?? "https://localhost:8443"

let refreshing = null  // singleton promise to avoid parallel refresh calls

export function getAccessToken() {
    return localStorage.getItem("access_token")
}

export async function refreshAccessToken() {
    if (refreshing) return refreshing

    refreshing = (async () => {
        const refreshToken = localStorage.getItem("refresh_token")
        if (!refreshToken) {
            logout()
            return null
        }

        try {
            const resp = await fetch(`${AUTH_URL}/auth/refresh`, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ refresh_token: refreshToken }),
            })

            if (!resp.ok) {
                logout()
                return null
            }

            const data = await resp.json()
            localStorage.setItem("access_token", data.access_token)
            localStorage.setItem("refresh_token", data.refresh_token)
            if (data.username) localStorage.setItem("username", data.username)
            if (data.avatar_url) localStorage.setItem("avatar_url", data.avatar_url ?? "")
            return data.access_token
        } catch {
            logout()
            return null
        } finally {
            refreshing = null
        }
    })()

    return refreshing
}

// fetch wrapper: retries once after refreshing on 401
export async function apiFetch(url, options = {}) {
    const token = getAccessToken()
    const withAuth = (t) => ({
        ...options,
        headers: {
            ...(options.headers ?? {}),
            ...(t ? { Authorization: `Bearer ${t}` } : {}),
        },
    })

    let resp = await fetch(url, withAuth(token))

    if (resp.status === 401) {
        const newToken = await refreshAccessToken()
        if (!newToken) return resp  // refresh failed, caller sees 401
        resp = await fetch(url, withAuth(newToken))
    }

    return resp
}

// Returns a fresh access token, refreshing silently if it expires within 60 s
export async function getFreshToken() {
    const token = getAccessToken()
    if (!token) return null
    try {
        const payload = JSON.parse(atob(token.split(".")[1]))
        if ((payload.exp * 1000) - Date.now() < 60_000) {
            return await refreshAccessToken()
        }
    } catch { /* fall through */ }
    return token
}

// Clears all session data and redirects to /login
export function logout() {
    localStorage.removeItem("access_token")
    localStorage.removeItem("refresh_token")
    localStorage.removeItem("username")
    localStorage.removeItem("avatar_url")
    window.location.href = "/login"
}

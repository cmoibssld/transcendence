const AUTH_URL = import.meta.env.VITE_AUTH_URL ?? "https://localhost:8443"

// All requests use credentials: "include" so the browser sends the HttpOnly
// access_token cookie automatically. No manual token management needed.
export async function apiFetch(url, options = {}) {
    const resp = await fetch(url, { ...options, credentials: "include" })

    if (resp.status === 401) {
        // Try to silently refresh via the refresh_token cookie
        const refreshed = await fetch(`${AUTH_URL}/auth/refresh`, {
            method: "POST",
            credentials: "include",
        })
        if (!refreshed.ok) {
            logout()
            return resp
        }
        // Retry original request with the new access_token cookie
        return fetch(url, { ...options, credentials: "include" })
    }

    return resp
}

// No longer needed — cookies are sent automatically by the browser.
// Kept as a no-op so callers don't break during the transition.
export async function getFreshToken() {
    return null
}

// No longer readable — token is in an HttpOnly cookie.
export function getAccessToken() {
    return null
}

export async function refreshAccessToken() {
    const resp = await fetch(`${AUTH_URL}/auth/refresh`, {
        method: "POST",
        credentials: "include",
    })
    if (!resp.ok) {
        logout()
        return null
    }
    const data = await resp.json()
    if (data.username) localStorage.setItem("username", data.username)
    if (data.avatar_url) localStorage.setItem("avatar_url", data.avatar_url ?? "")
    return "cookie" // signals success without exposing the token
}

export function logout() {
    sessionStorage.removeItem("auth_ok")
    fetch(`${AUTH_URL}/auth/logout`, {
        method: "POST",
        credentials: "include",
    }).finally(() => {
        localStorage.removeItem("username")
        localStorage.removeItem("avatar_url")
        localStorage.removeItem("user_id")
        window.location.href = "/login"
    })
}

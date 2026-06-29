const LOOPBACK_HOSTS = new Set(["localhost", "127.0.0.1", "::1"])

function isLoopbackHostname(hostname) {
    return LOOPBACK_HOSTS.has(hostname)
}

export function resolveHttpUrl(configuredUrl, fallbackPath = "") {
    if (typeof window === "undefined") {
        return configuredUrl ?? fallbackPath
    }

    const configured = (configuredUrl ?? "").trim()
    if (configured) {
        try {
            const parsed = new URL(configured)
            if (isLoopbackHostname(window.location.hostname) && isLoopbackHostname(parsed.hostname)) {
                return configured
            }
        } catch {
            return configured
        }
    }

    const resolved = new URL(fallbackPath, window.location.origin).toString()
    return fallbackPath ? resolved : resolved.replace(/\/$/, "")
}

export function resolveWsUrl(configuredUrl, fallbackPath = "/ws") {
    const resolved = resolveHttpUrl(configuredUrl, fallbackPath)
    if (resolved.startsWith("https://")) {
        return `wss://${resolved.slice("https://".length)}`
    }
    if (resolved.startsWith("http://")) {
        return `ws://${resolved.slice("http://".length)}`
    }
    return resolved
}
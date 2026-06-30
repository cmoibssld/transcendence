# Architecture Technique — Luigi Jack Casino

---

## Table des matières

1. [Architecture Globale](#1-architecture-globale)
   - 1.1 [Vue d'ensemble des microservices](#11-vue-densemble-des-microservices)
   - 1.2 [Orchestration Docker](#12-orchestration-docker)
   - 1.3 [Reverse Proxy nginx](#13-reverse-proxy-nginx)
   - 1.4 [Chaîne de démarrage et healthchecks](#14-chaîne-de-démarrage-et-healthchecks)
2. [Sécurité & Authentification](#2-sécurité--authentification)
   - 2.1 [Cycle de vie complet d'une session](#21-cycle-de-vie-complet-dune-session)
   - 2.2 [JWT Access Tokens](#22-jwt-access-tokens)
   - 2.3 [Refresh Tokens — stockage et rotation](#23-refresh-tokens--stockage-et-rotation)
   - 2.4 [Hachage des mots de passe](#24-hachage-des-mots-de-passe)
   - 2.5 [OAuth 2.0 GitHub](#25-oauth-20-github)
   - 2.6 [Validation des tokens par les services downstream](#26-validation-des-tokens-par-les-services-downstream)
   - 2.7 [Authentification inter-services](#27-authentification-inter-services)
   - 2.8 [Rate limiting](#28-rate-limiting)
   - 2.9 [Politique CORS](#29-politique-cors)
3. [Flux de Données & Réseau](#3-flux-de-données--réseau)
   - 3.1 [Connexion WebSocket — workflow complet pas à pas](#31-connexion-websocket--workflow-complet-pas-à-pas)
   - 3.2 [Routage des messages clients](#32-routage-des-messages-clients)
   - 3.3 [Push serveur — mécanisme de dispatch](#33-push-serveur--mécanisme-de-dispatch)
   - 3.4 [Flux d'une mise (game.bet) — atomicité des coins](#34-flux-dune-mise-gamebet--atomicité-des-coins)
   - 3.5 [Flux du chat — rejoindre une salle et envoyer un message](#35-flux-du-chat--rejoindre-une-salle-et-envoyer-un-message)
   - 3.6 [Gestion des déconnexions inattendues](#36-gestion-des-déconnexions-inattendues)
   - 3.7 [Catalogue des payloads WebSocket](#37-catalogue-des-payloads-websocket)
4. [Modèle de Données](#4-modèle-de-données)
   - 4.1 [Base `transcendance_oauth`](#41-base-transcendance_oauth)
   - 4.2 [Base `transcendance_history`](#42-base-transcendance_history)
   - 4.3 [Pools de connexions Diesel](#43-pools-de-connexions-diesel)
   - 4.4 [Historique des migrations](#44-historique-des-migrations)

---

## 1. Architecture Globale

### 1.1 Vue d'ensemble des microservices

Le projet est une pile de microservices entièrement écrits en Rust (sauf le frontend), orchestrés par Docker Compose. Chaque service est un binaire `actix-web` indépendant avec son propre `Cargo.lock` (pas de workspace Rust partagé).

| Service | Crate/Tech | Port interne | Port exposé hôte | Rôle |
|---|---|---|---|---|
| `db_oauth` | PostgreSQL 16 | 5432 | **5432** | Stockage utilisateurs, tokens, friendships |
| `db_history` | PostgreSQL 16 | 5432 | **5433** | Historique des parties et statistiques |
| `api` | Rust (runner Diesel) | 3443 (TLS) | 3443 | Exécution des migrations OAuth au démarrage |
| `oauth` | `actix-web` 4 | 8081 | **8081** | Auth, sessions, gestion des utilisateurs |
| `game-engine` | `actix-web` 4 | 8080 | **8085** | Logique de table, statistiques, historique |
| `ws-service` | `actix-web` 4 + `actix` | 8082 | *(non exposé)* | Gateway WebSocket authentifié |
| `chat` | `actix-web` 4 + `actix` | 8090 | *(non exposé)* | Salles de chat en mémoire |
| `frontend` | React 19 / Vite (Node) | 3000 | **3000** | SPA React |
| `reverse-proxy` | nginx | 80/443 | **8080**, **8443** | TLS termination, routage HTTP/WS |

**Réseau Docker** : tous les services partagent le réseau bridge `transcendance`. Les appels inter-services utilisent les noms de conteneur Docker comme hostnames (ex. `http://oauth:8081`, `http://game-engine:8080`).

---

### 1.2 Orchestration Docker

#### Variables d'environnement critiques

Définies dans `.env` ou exportées dans le shell avant `docker compose up` :

```
GITHUB_ID               — Client ID de l'app GitHub OAuth
GITHUB_SECRET           — Client Secret de l'app GitHub OAuth
JWT_SECRET              — Secret HMAC-SHA256 pour signer les JWT
REFRESH_TOKEN_SECRET    — Secret HMAC-SHA256 pour hacher les refresh tokens
SESSION_KEY             — Clé 64+ octets pour actix-session (CSRF OAuth)
OAUTH_REDIRECT_URI      — http://localhost:8081/auth/github/callback
FRONTEND_REDIRECT_URL   — http://localhost:3000/auth/callback
COOKIE_SECURE           — false (dev) / true (prod)
INTERNAL_SECRET         — Secret partagé pour les appels inter-services
```

Le `docker-compose.yml` fournit des valeurs par défaut de développement pour tout sauf `GITHUB_ID` et `GITHUB_SECRET` (via la syntaxe `${VAR:-default}`).

#### Dev overlay

`docker-compose.dev.yml` remplace les images statiques par des conteneurs avec **live-reload** :
- `frontend` : Vite dev server sur `:5173`, volume monté `./front-end:/app`
- `game-engine` : volume monté `./game-engine:/usr/src/game-engine` avec `RUST_LOG=debug`
- `chat` : volume monté `./chat:/usr/src/chat`

```bash
# Stack complète (images buildées)
docker compose up --build

# Overlay dev (live-reload)
docker compose -f docker-compose.yml -f docker-compose.dev.yml up
```

---

### 1.3 Reverse Proxy nginx

**Fichier** : `reverse-proxy/conf/nginx.conf`

nginx joue trois rôles : redirection HTTP→HTTPS, terminaison TLS, et routage vers les services upstream.

#### Configuration TLS

```nginx
ssl_certificate     /etc/nginx/certs/dev-cert.pem;
ssl_certificate_key /etc/nginx/certs/dev-key.pem;
ssl_protocols       TLSv1.2 TLSv1.3;
ssl_prefer_server_ciphers off;
```

Les certificats sont auto-signés (générés par `reverse-proxy/certs/gen-dev-certs.sh`) et montés en read-only via un volume Docker. En production, ces fichiers seraient remplacés par des certificats Let's Encrypt.

#### Upstreams déclarés

```nginx
upstream game_engine  { server game-engine:8080; }
upstream chat_service { server chat:8090; }
upstream frontend_app { server frontend:3000; }
upstream ws_service   { server ws-service:8082; }
upstream user_mgmt    { server oauth:8081; }
```

#### Table de routage

| Chemin nginx | Upstream | Notes |
|---|---|---|
| `GET /ws` | `ws_service/ws` | Upgrade WebSocket (voir §3.1) |
| `/auth/*` | `user_mgmt` (oauth:8081) | Toutes les routes auth |
| `/api/game/*` | `game_engine/` (strip `/api/game`) | REST game engine |
| `/api/chat/*` | `chat_service/` (strip `/api/chat`) | REST chat |
| `/` | `frontend_app` (frontend:3000) | SPA React |

#### Gestion de l'upgrade WebSocket

```nginx
map $http_upgrade $connection_upgrade {
    default  upgrade;
    ''       close;
}

location /ws {
    proxy_pass         http://ws_service/ws;
    proxy_http_version 1.1;
    proxy_set_header   Upgrade    $http_upgrade;
    proxy_set_header   Connection $connection_upgrade;
    proxy_set_header   Host       $host;
    proxy_set_header   X-Real-IP  $remote_addr;
    proxy_read_timeout 3600s;   # Maintient la connexion 1 heure
}
```

La directive `proxy_read_timeout 3600s` est critique : elle empêche nginx de fermer les connexions WebSocket idle après le timeout TCP par défaut de 60s.

#### CORS headers au niveau nginx

```nginx
add_header Access-Control-Allow-Origin  "http://localhost:3000" always;
add_header Access-Control-Allow-Methods "GET, POST, OPTIONS"    always;
add_header Access-Control-Allow-Headers "Content-Type, Authorization, X-Chat-User" always;
add_header Access-Control-Allow-Credentials "true" always;
if ($request_method = OPTIONS) { return 204; }
```

---

### 1.4 Chaîne de démarrage et healthchecks

L'ordre de démarrage est déterminé par les `depends_on` avec `condition: service_healthy` :

```
db_oauth  ──(healthy)──► api ──(healthy)──► oauth ──(healthy)──► frontend
                                             │                   
                                             └──(healthy)──► ws-service
                                                               ▲
db_history ──(healthy)──► game-engine ──(healthy)─────────────┘
                                                               ▲
chat ──(service_started)───────────────────────────────────────┘

Tous ──(various)──► reverse-proxy
```

| Service | Healthcheck | Intervalle | Retries |
|---|---|---|---|
| `db_oauth` / `db_history` | `pg_isready -U diesel -d <db>` | 5s | 10 |
| `api` | `curl -sfk https://localhost:3443/health` | 10s | 5 |
| `oauth` | `curl -sf http://localhost:8081/health` | 10s | 5 |
| `game-engine` | `curl -sf http://localhost:8080/health` | 10s | 5 |

Le service `api` (runner de migrations Diesel) doit être healthy avant qu'`oauth` démarre, garantissant que le schéma de la base `transcendance_oauth` est à jour avant que le service auth accepte des requêtes.

---

## 2. Sécurité & Authentification

Le service `oauth` (port 8081) est le seul détenteur de l'identité utilisateur. Aucun autre service ne stocke de credentials — tous délèguent la validation au service oauth via le endpoint `POST /auth/introspect`.

**Crates Rust utilisées** :
- `jsonwebtoken` — génération et validation des JWT (HS256)
- `bcrypt` — hachage des mots de passe
- `hmac` + `sha2` — HMAC-SHA256 pour les refresh tokens et le secret interne
- `actix-session` avec `CookieSessionStore` — stockage de l'état CSRF OAuth
- `rand` — génération de tokens aléatoires cryptographiquement sûrs

---

### 2.1 Cycle de vie complet d'une session

```
[Inscription / Login]
    │
    ▼
 POST /auth/login  ──► bcrypt::verify(password, hash)
    │
    ├── OK ──► issue_tokens(user_id, username, avatar)
    │              ├── generate_access_token()  → JWT HS256, 15min
    │              └── generate_refresh_token() → 64 chars aléatoires
    │                      └── hash_refresh_token() → HMAC-SHA256
    │                              └── INSERT refresh_tokens(token_hash, expires_at=+30j)
    │
    └── → { access_token, refresh_token, username, avatar_url }

[Utilisation de l'API]
    │
    ▼
 Authorization: Bearer <access_token>
    │
    ├── Validation locale (jwt::decode, pas de DB)
    │       └── OK → { user_id, username } extraits des claims
    │
    └── Expiration (401) → POST /auth/refresh { refresh_token }
            ├── hash_refresh_token(token) → cherche en DB
            ├── Vérifie: revoked_at IS NULL, expires_at > NOW()
            ├── UPDATE refresh_tokens SET revoked_at = NOW()  (révocation)
            └── issue_tokens() → nouveaux access_token + refresh_token

[Déconnexion]
    │
    ▼
 POST /auth/logout { refresh_token }
    └── UPDATE refresh_tokens SET revoked_at = NOW()
```

---

### 2.2 JWT Access Tokens

**Algorithme** : HMAC-SHA256 (HS256) via la crate `jsonwebtoken`.

**Structure des claims** :

```rust
#[derive(Debug, Serialize, Deserialize)]
struct ClaimsJWT {
    sub:   u64,    // user_id (i32 casté en u64)
    login: String, // username
    exp:   usize,  // Unix timestamp d'expiration
}
```

**Génération** :

```rust
fn generate_access_token(user_id: u64, username: String) -> Result<String, Error> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::minutes(15))
        .expect("Timestamp invalide")
        .timestamp() as usize;

    let claims = ClaimsJWT { sub: user_id, login: username, exp: expiration };

    encode(
        &Header::default(),  // alg: HS256
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
}
```

**Durée de vie** : **15 minutes** exactement. Passée cette durée, le frontend doit utiliser le refresh token pour obtenir un nouveau pair.

**Validation** (dans `introspect`) :

```rust
decode::<ClaimsJWT>(
    &token,
    &DecodingKey::from_secret(jwt_secret.as_bytes()),
    &Validation::default(),  // vérifie exp automatiquement
)
```

`Validation::default()` vérifie : signature HMAC, expiration (`exp`), et le format du token. Aucune validation de l'audience (`aud`) ou de l'émetteur (`iss`) n'est configurée.

---

### 2.3 Refresh Tokens — stockage et rotation

Les refresh tokens ne sont **jamais stockés en clair** en base de données.

**Génération du token raw** :

```rust
fn generate_refresh_token() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}
```

Produit une chaîne de 64 caractères alphanumériques aléatoires (entropie ≈ 381 bits pour le jeu `[A-Za-z0-9]`).

**Hachage avant stockage** :

```rust
type HmacSha256 = Hmac<Sha256>;

fn hash_refresh_token(token: &str) -> Result<String, Error> {
    let secret = env::var("REFRESH_TOKEN_SECRET")?;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())?;
    mac.update(token.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}
```

Le hash HMAC-SHA256 (64 caractères hexadécimaux) est stocké dans la colonne `token_hash` de `refresh_tokens`. Pour valider un token présenté par le client, on recalcule le HMAC et on compare avec `filter(token_hash.eq(&hash))`.

**Schéma de la table** :

```sql
CREATE TABLE refresh_tokens (
    id         BIGSERIAL   PRIMARY KEY,
    user_id    INT         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT        NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,        -- +30 jours
    revoked_at TIMESTAMPTZ NULL,            -- NULL = valide
    user_agent TEXT        NULL,            -- audit
    ip_address TEXT        NULL             -- audit
);
```

**Rotation** : à chaque appel à `POST /auth/refresh`, le token présenté est **immédiatement révoqué** (`revoked_at = NOW()`), et un nouveau pair `(access_token, refresh_token)` est émis et stocké. Il n'existe donc jamais deux tokens valides simultanément pour la même session.

**Métadonnées de session** : `user_agent` et `ip_address` sont extraits de la requête via :

```rust
fn extract_client_info(req: &HttpRequest) -> (Option<String>, Option<String>) {
    let user_agent = req.headers().get("User-Agent")...;
    let ip_address = req.connection_info().realip_remote_addr()...;
    (user_agent, ip_address)
}
```

`realip_remote_addr()` respecte l'en-tête `X-Real-IP` positionné par nginx.

---

### 2.4 Hachage des mots de passe

**Algorithme** : bcrypt via la crate `bcrypt`, coût `DEFAULT_COST` (12 rounds).

```rust
let hashed_password = hash(&user_data.password, DEFAULT_COST)?;
// Stockage dans users.password_hash

// Vérification
let is_valid = verify(&login_data.password, &user.password_hash).unwrap_or(false);
```

Les comptes GitHub (`provider = 'github'`) ont `password_hash = ""` et sont rejetés explicitement à la tentative de login local :

```rust
if user.provider != "local" {
    return HttpResponse::Forbidden()
        .json(json!({"error": "Utilisez GitHub pour vous connecter"}));
}
```

---

### 2.5 OAuth 2.0 GitHub

**Flow** : Authorization Code Grant avec protection CSRF via état de session.

**Étape 1 — Génération du state CSRF** (`GET /auth/github/login`) :

```rust
let state: String = rand::thread_rng()
    .sample_iter(&Alphanumeric)
    .take(32)
    .map(char::from)
    .collect();

session.insert("oauth_state", &state)?;  // stocké dans le cookie signé actix-session
```

Cookie configuré avec `SameSite::Lax` et `COOKIE_SECURE` (env). La clé de session (`SESSION_KEY`, 64+ octets) est utilisée pour signer le cookie via `actix-session::SessionMiddleware` avec `CookieSessionStore`.

Redirection vers :
```
https://github.com/login/oauth/authorize
  ?client_id={GITHUB_ID}
  &scope=read:user
  &state={state_32chars}
```

**Étape 2 — Callback** (`GET /auth/github/callback?code=…&state=…`) :

1. **Validation CSRF** : compare le `state` du query param avec `session.get::<String>("oauth_state")` — rejet si mismatch.
2. **Échange du code** : POST `https://github.com/login/oauth/access_token` avec `(client_id, client_secret, code)`, header `Accept: application/json`.
3. **Récupération du profil** : GET `https://api.github.com/user` avec `Authorization: Bearer <github_token>` et `User-Agent: ft_transcendence`.
4. **Upsert en base** :
   ```rust
   diesel::insert_into(users_dsl::users)
       .values(&new_user)
       .on_conflict(users_dsl::email)   // email = login@github.local
       .do_update()
       .set((
           users_dsl::username.eq(excluded(users_dsl::username)),
           users_dsl::avatar.eq(excluded(users_dsl::avatar)),
       ))
       .returning(User::as_returning())
       .get_result(&mut conn)
   ```
   L'email synthétique est `{login}@github.local` pour garantir l'unicité sans collecter l'email réel (qui peut être privé sur GitHub).

**Étape 3 — One-time code** :

Pour transmettre les tokens au frontend sans les exposer dans l'URL de redirection, un code à usage unique est généré :

```rust
// CodeStore = Mutex<HashMap<String, (AuthResponse, Instant)>>
let one_time_code: String = rand::thread_rng()
    .sample_iter(&Alphanumeric).take(32)...; 
store.lock().unwrap().insert(one_time_code.clone(), (response_data, Instant::now()));

// Redirection vers le frontend
redirect_url = format!("{}?code={}", FRONTEND_REDIRECT_URL, urlencoding::encode(&one_time_code));
```

Le frontend appelle ensuite `GET /auth/exchange?code=<code>` qui :
- Évince les codes expirés (> 60s) à chaque appel
- Retire et retourne les tokens (consommation unique)
- Retourne `400` si le code est invalide ou expiré

---

### 2.6 Validation des tokens par les services downstream

Aucun service downstream (game-engine, ws-service) ne valide lui-même la signature JWT. Ils délèguent à `POST /auth/introspect` :

**Requête** :
```json
{ "access_token": "<jwt>" }
```
ou header `Authorization: Bearer <jwt>`.

**Réponse** :
```json
{ "user_id": 42, "username": "alice" }
```

Le endpoint `introspect` accepte les deux formes pour permettre aux services internes de choisir leur mécanisme de transport. Il ne fait aucune requête en base — la validation est entièrement JWT (stateless) : `jsonwebtoken::decode` vérifie signature + `exp`.

**Implémentation dans ws-service** (`ws_index`) :

```rust
let response = state.http_client
    .post(format!("{}/auth/introspect", state.user_service_url))
    .json(&IntrospectRequest { access_token: query.token.clone() })
    .send()
    .await
    .map_err(|_| ErrorUnauthorized("Auth failed"))?;

if !response.status().is_success() {
    return Err(ErrorUnauthorized("Auth failed"));
}

let auth = response.json::<IntrospectResponse>().await?;
// → auth.user_id, auth.username
```

**Implémentation dans game-engine** (`get_history`, `get_stats_me`, `get_friend_stats`) :

```rust
async fn introspect_token(client: &reqwest::Client, user_url: &str, token: &str) -> Option<i64> {
    let resp = client
        .post(format!("{}/auth/introspect", user_url))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.ok()?;
    if !resp.status().is_success() { return None; }
    let body: serde_json::Value = resp.json().await.ok()?;
    body.get("user_id")?.as_i64()
}
```

---

### 2.7 Authentification inter-services

Les appels entre services internes (game-engine → oauth, chat → ws-service) utilisent un **secret partagé** transmis dans l'en-tête `X-Internal-Secret`.

**Validation côté oauth** :

```rust
fn check_internal_secret(req: &HttpRequest) -> Option<HttpResponse> {
    let secret = env::var("INTERNAL_SECRET")?;
    check_internal_secret_with(req, &secret)
}

fn check_internal_secret_with(req: &HttpRequest, secret: &str) -> Option<HttpResponse> {
    let provided = req.headers()
        .get("X-Internal-Secret")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !secrets_equal(provided, secret) {
        return Some(HttpResponse::Unauthorized()...);
    }
    None
}
```

**Comparaison en temps constant** (`secrets_equal`) pour prévenir les attaques timing :

```rust
fn secrets_equal(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() { return false; }
    let mut diff = 0u8;
    for (&x, &y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;  // XOR accumulé, pas de court-circuit
    }
    diff == 0
}
```

Routes protégées par ce mécanisme :

| Route | Appelant |
|---|---|
| `POST /auth/internal/coins` | game-engine (déduction/remboursement de mise) |
| `POST /auth/internal/users` | game-engine (résolution username/avatar pour leaderboard) |
| `POST /auth/internal/friend-check` | game-engine (vérification d'amitié pour stats) |

---

### 2.8 Rate Limiting

Appliqué uniquement sur `POST /auth/login` pour limiter les attaques par force brute.

**Implémentation** : `LoginAttempts = Mutex<HashMap<String, (u32, Instant)>>` — état en mémoire partagé via `web::Data`.

```rust
fn check_rate_limit(attempts: &web::Data<LoginAttempts>, ip: &str) -> bool {
    let mut map = attempts.lock().unwrap();
    let now = Instant::now();
    let entry = map.entry(ip.to_string()).or_insert((0, now));
    if entry.1.elapsed() > Duration::from_secs(60) {
        *entry = (1, now);   // fenêtre expirée → reset
        return true;
    }
    if entry.0 >= 5 { return false; }
    entry.0 += 1;
    true
}
```

- **Seuil** : 5 tentatives par IP par fenêtre de 60 secondes
- **Clé** : adresse IP via `req.connection_info().realip_remote_addr()` (respecte `X-Real-IP` de nginx)
- **Réponse en cas de dépassement** : `429 Too Many Requests`
- **Limitation** : pas de persistence entre redémarrages du conteneur (état en RAM uniquement)

---

### 2.9 Politique CORS

Configurée via la crate `actix-cors` dans les services qui exposent des routes publiques.

**oauth** :
```rust
Cors::default()
    .allowed_origin("http://localhost:3000")
    .allowed_origin("http://localhost:5173")
    .allowed_origin("https://localhost:8443")
    .allowed_methods(vec!["GET", "POST", "PATCH", "OPTIONS"])
    .allowed_headers(vec![header::CONTENT_TYPE, header::AUTHORIZATION, header::ACCEPT])
    .supports_credentials()
```

**game-engine** :
```rust
Cors::default()
    .allowed_origin(&frontend_origin)  // FRONTEND_ORIGIN env var, défaut localhost:3000
    .allowed_methods(vec!["GET", "POST", "OPTIONS"])
    .allowed_headers(vec![header::CONTENT_TYPE, header::AUTHORIZATION])
```

**chat** :
```rust
Cors::default()
    .allowed_origin("http://localhost:3000")
    .allowed_methods(vec!["GET", "POST", "OPTIONS"])
    .allowed_headers(vec![
        header::CONTENT_TYPE,
        header::AUTHORIZATION,
        header::ACCEPT,
        HeaderName::from_static("x-chat-user"),  // header custom
    ])
    .supports_credentials()
```

---

## 3. Flux de Données & Réseau

### 3.1 Connexion WebSocket — workflow complet pas à pas

La connexion WebSocket est établie à travers trois composants en série : nginx, ws-service, et le système d'acteurs `actix`.

**Étape 1 : Le client initie la connexion**

```
Browser → wss://localhost:8443/ws?token=<jwt_access_token>
```

Le token JWT (15 min de validité) est passé en query string car les navigateurs n'autorisent pas les headers custom dans l'API WebSocket standard.

**Étape 2 : nginx effectue l'upgrade HTTP → WebSocket**

nginx reçoit la requête HTTP `GET /ws?token=...` avec les headers :
```
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Key: <base64>
Sec-WebSocket-Version: 13
```

Grace au bloc `map $http_upgrade $connection_upgrade`, nginx transmet l'upgrade à ws-service avec `proxy_http_version 1.1` et ajoute `X-Real-IP`. Le timeout est étendu à `3600s` pour les connexions longue durée.

**Étape 3 : Authentification dans `ws_index`**

```rust
async fn ws_index(
    req:    HttpRequest,
    stream: web::Payload,
    state:  web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    // 1. Extraire le token du query string
    let query = web::Query::<WsQuery>::from_query(req.query_string())
        .map_err(|_| ErrorBadRequest("Missing token"))?;

    // 2. Appel synchrone (dans async context) à oauth/auth/introspect
    let url = format!("{}/auth/introspect", state.user_service_url);
    let response = state.http_client
        .post(url)
        .json(&IntrospectRequest { access_token: query.token.clone() })
        .send().await
        .map_err(|_| ErrorUnauthorized("Auth failed"))?;

    // 3. Rejet si non-200
    if !response.status().is_success() {
        return Err(ErrorUnauthorized("Auth failed"));
    }

    // 4. Extraction du user_id
    let auth = response.json::<IntrospectResponse>().await
        .map_err(|_| ErrorUnauthorized("Auth failed"))?;

    // 5. Upgrade HTTP → WebSocket et création de l'acteur
    let session = WsSession::new(auth.user_id, state.get_ref().clone());
    ws::start(session, &req, stream)
}
```

**Étape 4 : Enregistrement de la session dans `Actor::started`**

`ws::start()` démarre l'acteur `WsSession`. La méthode `started()` est appelée immédiatement :

```rust
fn started(&mut self, ctx: &mut Self::Context) {
    let mut clients = self.state.clients.lock().unwrap();
    clients.insert(self.user_id, ctx.address());
    // clients : web::Data<Mutex<HashMap<u64, actix::Addr<WsSession>>>>
}
```

La `Addr<WsSession>` est l'identifiant d'acteur `actix`. Elle permet d'envoyer des messages à la session depuis n'importe quel thread via `addr.do_send(msg)` — la communication est thread-safe par le système de mailbox de `actix`.

**Étape 5 : La connexion est établie**

Le navigateur reçoit la réponse HTTP `101 Switching Protocols`. Le handshake WebSocket est complété. La connexion est maintenant une connexion duplex persistante gérée par le runtime `actix-web::rt` (tokio).

---

### 3.2 Routage des messages clients

Chaque message envoyé par le client est un JSON avec la forme :

```json
{ "kind": "<namespace>.<action>", "payload": { ... } }
```

Le `StreamHandler<ws::Message>` de `WsSession` route selon le préfixe du `kind` :

```rust
fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
    match item {
        Ok(ws::Message::Text(text)) => {
            let payload = serde_json::from_str::<ClientEvent>(&text);

            actix_web::rt::spawn(async move {
                let target_url = if payload.kind.starts_with("game.") {
                    format!("{}/ws/ingest", state.game_service_url)
                } else if payload.kind.starts_with("chat.") {
                    format!("{}/ws/message", state.chat_service_url)
                } else {
                    return; // presence.* et autres : traitement local
                };

                state.http_client
                    .post(target_url)
                    .json(&ForwardEvent { user_id, kind: payload.kind, payload: payload.payload })
                    .send().await;
            });
        }
        Ok(ws::Message::Ping(p)) => ctx.pong(&p),
        Ok(ws::Message::Close(reason)) => { ctx.close(reason); ctx.stop(); }
        Err(_) => ctx.stop(),
        _ => {}
    }
}
```

**Points importants** :
- Le forwarding est **fire-and-forget** (les erreurs de `send().await` sont ignorées)
- Chaque message est traité dans une tâche tokio séparée (`rt::spawn`) — les messages sont traités en parallèle, sans garantie d'ordre
- Le `ForwardEvent` transmis aux services downstream contient `user_id` (authentifié, non falsifiable côté client)

**Payload type `ForwardEvent`** (défini dans `ws_service/src/api.rs`) :

```rust
pub struct ForwardEvent {
    pub user_id: u64,
    pub kind:    String,
    pub payload: Value,   // serde_json::Value
}
```

---

### 3.3 Push serveur — mécanisme de dispatch

Les services backend (game-engine, chat) envoient des événements aux clients via `POST /dispatch` sur ws-service :

**Requête** :
```json
{
  "user_ids": [42, 17, 99],
  "event":    { "kind": "game.state", "payload": { ... } }
}
```

**Handler** :

```rust
async fn dispatch_event(
    state: web::Data<AppState>,
    body:  web::Json<DispatchRequest>,
) -> impl Responder {
    let clients = state.clients.lock().unwrap();
    for user_id in &body.user_ids {
        if let Some(addr) = clients.get(user_id) {
            addr.do_send(ServerEvent(body.event.clone()));
        }
        // Si l'utilisateur n'est pas connecté → ignoré silencieusement
    }
    HttpResponse::Ok().json(json!({ "status": "accepted" }))
}
```

`addr.do_send()` dépose le message dans la mailbox de l'acteur sans bloquer. L'acteur le traite dans son propre thread via `Handler<ServerEvent>` :

```rust
impl Handler<ServerEvent> for WsSession {
    type Result = ();
    fn handle(&mut self, msg: ServerEvent, ctx: &mut Self::Context) {
        if let Ok(text) = serde_json::to_string(&msg.0) {
            ctx.text(text);   // envoie le JSON sérialisé sur le WebSocket
        }
    }
}
```

**Types définis dans `ws_service/src/api.rs`** :

```rust
pub struct OutboundEvent {
    pub kind:    String,
    pub payload: Value,
}

pub struct DispatchRequest {
    pub user_ids: Vec<u64>,
    pub event:    OutboundEvent,
}

// Message interne actix
struct ServerEvent(OutboundEvent);  // impl actix::Message
```

---

### 3.4 Flux d'une mise (game.bet) — atomicité des coins

Le système garantit qu'aucun coin n'est perdu ni créé sans contrepartie via un pattern **déduire avant d'enregistrer, rembourser en cas de rejet**.

```
Client ──WS──► ws-service
                   │
                   ▼ POST /ws/ingest { user_id, kind: "game.bet", payload: { amount: 100 } }
                   │
              game-engine
                   │
                   ├─1─► Extraire bet_amount = payload.amount (100)
                   │
                   ├─2─► POST oauth:8081/auth/internal/coins
                   │         { user_id: 42, delta: -100 }
                   │         X-Internal-Secret: <secret>
                   │
                   │         oauth vérifie: SELECT money FROM users WHERE id=42
                   │         Si money + delta < 0 → 400 "insufficient funds"
                   │         Sinon → UPDATE users SET money = money - 100 → 200
                   │
                   ├─3a─► Si déduction échoue:
                   │         dispatch(ws-service, [user_id], game.error { "Solde insuffisant" })
                   │         return 202
                   │
                   ├─3b─► Si déduction OK:
                   │         result = game.handle_ingest(user_id, "game.bet", payload)
                   │
                   ├─4─► Si game-engine rejette la mise (phase incorrecte, etc.):
                   │         result.personal.kind == "game.error"
                   │         → POST oauth/auth/internal/coins { delta: +100 }  (remboursement)
                   │
                   └─5─► Diffusion du résultat:
                             dispatch(ws-service, audience, game.state)
                             dispatch(ws-service, [user_id], personal_event)
```

**Le même pattern s'applique à `game.double`**, avec une différence : le montant à déduire n'est pas dans le payload (le client n'envoie pas `{ amount }`) mais est récupéré depuis l'état du jeu en mémoire via `game.get_player_bet(user_id)` avant l'appel à `handle_ingest`.

**Post-broadcast différé** : après un `game.round_end`, le nouveau `game.state` (phase de mise suivante) est envoyé avec un délai de **2500ms** via `tokio::time::sleep`, permettant à l'animation côté frontend de se terminer avant le reset de l'interface :

```rust
actix_web::rt::spawn(async move {
    tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
    dispatch(&c2, &ws, aud, post).await;
});
```

---

### 3.5 Flux du chat — rejoindre une salle et envoyer un message

Le chat utilise deux chemins parallèles pour les connexions :
- **`ws_clients`** : connexions WebSocket directes à chat:8090 (non utilisé dans l'architecture actuelle via le gateway)
- **`ws_users`** : utilisateurs connectés via ws-service (chemin principal)

**Rejoindre une salle** (`chat.join` via ws-service) :

```
Client ──WS──► ws-service
                   │ POST chat:8090/ws/message
                   │ { user_id: 42, kind: "chat.join", payload: { room_id: "table-1", name: "Alice" } }
                   │
              chat (ws_message handler)
                   │
                   ├─1─► hub.join_rest("table-1", 42, "Alice")
                   │         room.ws_users.insert(42)
                   │         room.ws_user_names.insert(42, "Alice")
                   │         returns (audience: [42, ...], history: Vec<ChatEnvelope>)
                   │
                   ├─2─► Envoyer l'historique au nouvel arrivant (jusqu'à 50 messages):
                   │         Pour chaque message dans history:
                   │             POST ws-service:8082/dispatch { user_ids: [42], event: msg }
                   │
                   ├─3─► Créer l'événement système:
                   │         ChatEnvelope { kind: "chat.system", user: "system",
                   │                        message: "Alice joined the chat" }
                   │
                   └─4─► hub.message_rest("table-1", system_event)
                             // NE PAS stocker en history (condition: kind == "chat.message" uniquement)
                             broadcast aux ws_clients locaux
                             POST ws-service:8082/dispatch { user_ids: audience }
```

**Envoyer un message** (`chat.message`) :

```
chat.message_rest(room_id, envelope):
    └── if envelope.kind == "chat.message":
            room.history.push_back(envelope)    // VecDeque, max 50 (pop_front si plein)
        for client in room.ws_clients.values():
            client.do_send(WsServerEvent(envelope))
        return room.ws_users  // audience pour dispatch via ws-service
```

**Structure `ChatEnvelope`** :

```rust
struct ChatEnvelope {
    client_id: Uuid,      // identifiant unique du message
    room_id:   String,    // ex: "table-1"
    kind:      String,    // "chat.message" | "chat.system"
    user:      String,    // nom d'affichage de l'émetteur
    message:   String,
    timestamp: u64,       // Unix seconds (SystemTime::now())
}
```

**Décision architecture critique** : les événements système (`chat.join`, `chat.leave`, `chat.rename`) **ne sont PAS persistés** dans `room.history`. Si l'historique les stockait, un utilisateur qui rejoint la salle après plusieurs cycles join/leave recevrait des messages de type "X joined" qui ne correspondent plus à la réalité de la salle. Seuls les `chat.message` sont rejoués à la connexion.

---

### 3.6 Gestion des déconnexions inattendues

Quand un WebSocket se ferme (drop de connexion, fermeture navigateur), `Actor::stopped()` est appelé dans ws-service :

```rust
fn stopped(&mut self, _ctx: &mut Self::Context) {
    // 1. Retirer du registre de sessions actives
    let mut clients = self.state.clients.lock().unwrap();
    clients.remove(&self.user_id);

    // 2. Notifier les services downstream de manière asynchrone
    let user_id  = self.user_id;
    let game_url = self.state.game_service_url.clone();
    let user_url = self.state.user_service_url.clone();

    actix_web::rt::spawn(async move {
        // Notifier game-engine (skip en Betting, auto-stand en Playing)
        let _ = client.post(format!("{}/ws/ingest", game_url))
            .json(&json!({ "user_id": user_id, "kind": "game.disconnect", "payload": {} }))
            .send().await;

        // Mettre à jour la présence (endpoint placeholder)
        let _ = client.post(format!("{}/auth/presence/offline", user_url))
            .json(&json!({ "user_id": user_id }))
            .send().await;
    });
}
```

**Comportement dans game-engine** (handler `"game.disconnect"` → `do_disconnect`) :
- **Si en phase Betting, mise non encore placée** : `skip_player()` → comme `game.skip`
- **Si en phase Betting, mise déjà placée** : `remove_player()`, vérifie si on peut passer à Playing
- **Si en phase Playing, c'est le tour du joueur** : `table.stand(player_id)` (auto-stand), résolution des coins
- **Si en phase Playing, tour d'un autre** : `remove_player()` silencieux

---

### 3.7 Catalogue des payloads WebSocket

#### Messages client → serveur (via ws-service)

| `kind` | Direction | Payload | Service cible |
|---|---|---|---|
| `game.join` | C→S | `{ table_id: number }` | game-engine |
| `game.bet` | C→S | `{ amount: number }` | game-engine |
| `game.hit` | C→S | `{}` | game-engine |
| `game.stay` | C→S | `{}` | game-engine |
| `game.double` | C→S | `{}` | game-engine |
| `game.skip` | C→S | `{}` | game-engine |
| `game.leave` | C→S | `{}` | game-engine |
| `chat.join` | C→S | `{ room_id: string, name: string }` | chat |
| `chat.leave` | C→S | `{ room_id: string }` | chat |
| `chat.message` | C→S | `{ room_id: string, message: string }` | chat |

#### Messages serveur → client (via ws-service dispatch)

| `kind` | Émetteur | Description |
|---|---|---|
| `game.joined` | game-engine | Confirmation de connexion à une table : `{ player_id, table_id }` |
| `game.state` | game-engine | État complet de la table (phase, cartes, mises, joueurs) |
| `game.round_end` | game-engine | Résultats de fin de manche : `{ game: GameState, result: RoundResult }` |
| `game.waiting` | game-engine | Aucune table disponible : `{ message }` |
| `game.left` | game-engine | Confirmation de départ de table |
| `game.error` | game-engine | Erreur : `{ message }` |
| `chat.message` | chat | Message utilisateur : `ChatEnvelope` |
| `chat.system` | chat | Événement système (join/leave/rename) : `ChatEnvelope` |

---

## 4. Modèle de Données

### 4.1 Base `transcendance_oauth`

Gérée par le service `oauth` via **Diesel ORM 2.x** avec le driver `diesel::PgConnection`. Le schéma est versionnémené par les migrations dans `db/migrations/`.

#### Schéma complet final

```sql
-- Table principale des utilisateurs
CREATE TABLE users (
    id            SERIAL       PRIMARY KEY,
    username      VARCHAR(50)  NOT NULL UNIQUE,
    email         VARCHAR(255) NOT NULL UNIQUE,
    password_hash TEXT         NOT NULL,           -- bcrypt hash ou "" pour GitHub
    money         BIGINT       NOT NULL DEFAULT 1000,
    avatar        TEXT         NOT NULL DEFAULT '', -- URL ou chaîne vide
    provider      VARCHAR(10)  NOT NULL DEFAULT 'local', -- 'local' | 'github'
    bio           TEXT         NOT NULL DEFAULT '',
    updated_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- Sessions de refresh token
CREATE TABLE refresh_tokens (
    id         BIGSERIAL   PRIMARY KEY,
    user_id    INT         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT        NOT NULL UNIQUE,   -- HMAC-SHA256 hex du token raw
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,          -- now() + 30 jours
    revoked_at TIMESTAMPTZ NULL,              -- NULL = token valide
    user_agent TEXT        NULL,
    ip_address TEXT        NULL
);
-- Index sur user_id et expires_at pour les requêtes de validation
CREATE INDEX refresh_tokens_user_id_idx   ON refresh_tokens(user_id);
CREATE INDEX refresh_tokens_expires_at_idx ON refresh_tokens(expires_at);

-- Graphe social (liens d'amitié)
CREATE TYPE friendship_status AS ENUM ('pending', 'accepted', 'declined');

CREATE TABLE friendships (
    id           SERIAL           PRIMARY KEY,
    requester_id INT              NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    addressee_id INT              NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status       friendship_status NOT NULL DEFAULT 'pending',
    created_at   TIMESTAMPTZ      NOT NULL DEFAULT NOW(),
    CONSTRAINT no_self_friend CHECK (requester_id != addressee_id),
    CONSTRAINT unique_pair    UNIQUE (requester_id, addressee_id)
);
```

#### Schéma Diesel (`oauth/src/schema.rs`)

```rust
diesel::table! {
    users (id) {
        id -> Int4,  username -> Varchar,  email -> Varchar,
        password_hash -> Text,  money -> Int8,  avatar -> Text,
        provider -> Varchar,  bio -> Text,  updated_at -> Timestamptz,
    }
}

diesel::table! {
    refresh_tokens (id) {
        id -> Int8,  user_id -> Int4,  token_hash -> Text,
        created_at -> Timestamptz,  expires_at -> Timestamptz,
        revoked_at -> Nullable<Timestamptz>,
        user_agent -> Nullable<Text>,  ip_address -> Nullable<Text>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::models::FriendshipStatusType;
    friendships (id) {
        id -> Int4,  requester_id -> Int4,  addressee_id -> Int4,
        status -> FriendshipStatusType,  created_at -> Timestamptz,
    }
}

diesel::joinable!(refresh_tokens -> users (user_id));
diesel::joinable!(friendships -> users (requester_id));
diesel::allow_tables_to_appear_in_same_query!(users, refresh_tokens, friendships);
```

#### Type ENUM PostgreSQL personnalisé

Le type `friendship_status` est un ENUM PostgreSQL natif. Diesel ne génère pas automatiquement le mapping pour les ENUMs PostgreSQL — il faut une implémentation manuelle via les traits `ToSql` et `FromSql` :

```rust
#[derive(SqlType)]
#[diesel(postgres_type(name = "friendship_status"))]
pub struct FriendshipStatusType;

// QueryId requis pour les ENUMs PostgreSQL — Diesel ne peut pas l'inférer statiquement
impl diesel::query_builder::QueryId for FriendshipStatusType {
    type QueryId = Self;
    const HAS_STATIC_QUERY_ID: bool = true;
}

impl ToSql<FriendshipStatusType, Pg> for FriendshipStatus {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let s = match self {
            FriendshipStatus::Pending  => "pending",
            FriendshipStatus::Accepted => "accepted",
            FriendshipStatus::Declined => "declined",
        };
        out.write_all(s.as_bytes())?;
        Ok(IsNull::No)
    }
}
```

#### Modèles Rust (`oauth/src/models.rs`)

```rust
#[derive(Queryable, Selectable, Identifiable, Debug, Clone)]
#[diesel(table_name = crate::schema::users)]
pub struct User {
    pub id: i32,  pub username: String,  pub email: String,
    pub password_hash: String,  pub money: i64,  pub avatar: String,
    pub provider: String,  pub bio: String,  pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::users)]
pub struct NewUser<'a> {
    pub username: &'a str,  pub email: &'a str,  pub password_hash: &'a str,
    pub money: i64,  pub avatar: &'a str,  pub provider: &'a str,
}

// Mise à jour partielle de profil — seuls les Some() sont écrits en DB
#[derive(AsChangeset)]
#[diesel(table_name = crate::schema::users)]
pub struct UserProfileChangeset {
    pub username:      Option<String>,
    pub avatar:        Option<String>,
    pub bio:           Option<String>,
    pub password_hash: Option<String>,
    pub updated_at:    DateTime<Utc>,   // toujours mis à jour
}
```

**Comportement de `AsChangeset` avec `Option`** : quand un champ est `None`, Diesel l'exclut du `SET` clause SQL. `updated_at` est `DateTime<Utc>` (non-`Option`) et est donc toujours inclus dans l'`UPDATE`.

#### Requêtes friendships notables

La liste des amis nécessite deux requêtes séparées car la contrainte `UNIQUE (requester_id, addressee_id)` ne crée pas de relation symétrique :

```rust
// Amis où l'utilisateur est requester
let sent: Vec<i32> = friendships
    .filter(requester_id.eq(me).and(status.eq(Accepted)))
    .select(addressee_id).load(&mut conn)?;

// Amis où l'utilisateur est addressee
let received: Vec<i32> = friendships
    .filter(addressee_id.eq(me).and(status.eq(Accepted)))
    .select(requester_id).load(&mut conn)?;

let friend_ids: Vec<i32> = sent.into_iter().chain(received).collect();
```

---

### 4.2 Base `transcendance_history`

Gérée par le service `game-engine`. Contrairement à `transcendance_oauth` dont les migrations sont gérées par un service `api` séparé, **game-engine gère ses propres migrations au démarrage** via la macro `embed_migrations!` de la crate `diesel_migrations` :

```rust
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

// Dans main() :
let mut conn = pool.get()?;
conn.run_pending_migrations(MIGRATIONS).expect("Migration failed");
```

Les migrations sont **compilées dans le binaire** — le service est auto-suffisant et applique le schéma à froid sans outil externe.

#### Schéma (inféré du code `game-engine/src/main.rs`)

```sql
-- Chaque manche jouée
CREATE TABLE game_rounds (
    id            BIGSERIAL   PRIMARY KEY,
    table_id      INT         NOT NULL,
    round_number  INT         NOT NULL,
    dealer_points INT         NOT NULL,
    result_json   TEXT        NOT NULL   -- JSON complet du résultat
);

-- Résultat individuel par joueur par manche
CREATE TABLE round_players (
    id        BIGSERIAL    PRIMARY KEY,
    round_id  BIGINT       NOT NULL REFERENCES game_rounds(id),
    user_id   BIGINT       NOT NULL,
    bet       BIGINT       NOT NULL,
    outcome   outcome_enum NOT NULL,     -- 'blackjack'|'win'|'push'|'loss'
    payout    BIGINT       NOT NULL,
    played_at TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- Statistiques agrégées par joueur (upsert incrémental)
CREATE TABLE user_stats (
    user_id       BIGINT  PRIMARY KEY,
    games_played  INT     NOT NULL DEFAULT 0,
    wins          INT     NOT NULL DEFAULT 0,
    losses        INT     NOT NULL DEFAULT 0,
    pushes        INT     NOT NULL DEFAULT 0,
    total_wagered BIGINT  NOT NULL DEFAULT 0,
    total_won     BIGINT  NOT NULL DEFAULT 0,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

#### Pattern d'upsert des statistiques

Après chaque manche, `save_round()` insère d'abord dans `game_rounds` et `round_players`, puis fait un upsert atomique dans `user_stats` :

```rust
diesel::insert_into(us::user_stats)
    .values(NewUserStats {
        user_id:       p.user_id as i64,
        games_played:  1,
        wins:          if is_win  { 1 } else { 0 },
        losses:        if is_loss { 1 } else { 0 },
        pushes:        if is_push { 1 } else { 0 },
        total_wagered: p.bet    as i64,
        total_won:     p.payout as i64,
    })
    .on_conflict(us::user_id)
    .do_update()
    .set((
        us::games_played .eq(us::games_played  + 1),
        us::wins         .eq(us::wins          + excluded(us::wins)),
        us::losses       .eq(us::losses        + excluded(us::losses)),
        us::pushes       .eq(us::pushes        + excluded(us::pushes)),
        us::total_wagered.eq(us::total_wagered + excluded(us::total_wagered)),
        us::total_won    .eq(us::total_won     + excluded(us::total_won)),
        us::updated_at   .eq(diesel::dsl::now),
    ))
    .execute(&mut conn)?;
```

`excluded(col)` est la syntaxe Diesel pour `EXCLUDED.column` en SQL PostgreSQL — elle référence la valeur de la ligne tentant d'être insérée.

#### Résolution des usernames pour le leaderboard

`user_stats` ne stocke pas les usernames (pour éviter la désynchronisation). Le leaderboard appelle `POST /auth/internal/users` avec la liste des `user_id` pour résoudre les noms en un seul batch :

```rust
async fn resolve_usernames(client, user_url, secret, user_ids: &[i64])
    -> HashMap<i64, (String, Option<String>)>
{
    client.post(format!("{}/auth/internal/users", user_url))
        .header("X-Internal-Secret", secret)
        .json(&json!({ "user_ids": user_ids }))
        .send().await
    // → Vec<{ id, username, avatar_url }>
}
```

---

### 4.3 Pools de connexions Diesel

Les deux services utilisent `diesel::r2d2` pour le pooling de connexions :

```rust
// oauth (pool plus grand car plus de routes concurrentes)
Pool::builder()
    .max_size(20)
    .connection_timeout(Duration::from_secs(5))
    .build(ConnectionManager::<PgConnection>::new(conn_str))

// game-engine
Pool::builder()
    .max_size(10)
    .connection_timeout(Duration::from_secs(5))
    .build(ConnectionManager::<PgConnection>::new(database_url))
```

Les handlers actix-web qui nécessitent la DB utilisent `web::block(move || { ... })` pour déporter les opérations bloquantes Diesel sur le thread pool de `actix_web::rt::Runtime`, évitant de bloquer le runtime tokio :

```rust
let result = web::block(move || {
    let mut conn = pool.get().expect("DB pool");
    users.filter(id.eq(user_id)).first::<User>(&mut conn)
}).await  // retourne Result<Result<T, diesel::Error>, BlockingError>
```

---

### 4.4 Historique des migrations

#### Base `transcendance_oauth` (service `db` / runner `api`)

| Migration | Timestamp | Changement |
|---|---|---|
| `0000_diesel_initial_setup` | auto | Fonctions helpers Diesel (`diesel_manage_updated_at`, `diesel_set_updated_at`) |
| `0000_init_users` | 2026-04-16 | Table `users` initiale : `id, username, email, password_hash, created_at` |
| `0001_user_money_only` | 2026-04-22 | Supprime `created_at`, ajoute `money BIGINT DEFAULT 0` |
| `0002_add_avatar_to_users` | 2026-04-24 | Ajoute `avatar TEXT DEFAULT ''` |
| `0001_refresh_tokens` | 2026-05-26 | Table `refresh_tokens` + index `user_id`, `expires_at` |
| `0002_default_coins` | 2026-05-26 | `UPDATE users SET money=1000 WHERE money=0` + `ALTER DEFAULT 1000` |
| `0003_add_provider_to_users` | 2026-05-28 | Ajoute `provider VARCHAR(10) DEFAULT 'local'`, backfill GitHub |
| `0004_friendships` | 2026-05-28 | ENUM `friendship_status`, table `friendships` avec contraintes `no_self_friend` et `unique_pair` |
| `0005_profile_fields` | 2026-05-28 | Ajoute `bio TEXT DEFAULT ''` et `updated_at TIMESTAMPTZ DEFAULT NOW()` |

**Note** : les migrations 0001–0004 de mai 2026 utilisent une numérotation sans date complète car elles ont été générées manuellement, pas via `diesel migration generate`.

#### Base `transcendance_history` (embarquée dans `game-engine`)

Les migrations de cette base sont stockées dans `game-engine/migrations/` et compilées dans le binaire via `embed_migrations!`. Elles sont appliquées automatiquement au démarrage du service, sans dépendance externe.

---

*Fin de la documentation technique.*

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

### Running the project

```bash
# Production stack (full build)
docker compose up --build

# Development overlay (live-reload for frontend, game-engine, chat)
docker compose -f docker-compose.yml -f docker-compose.dev.yml up
```

- Application: http://localhost:8080 (via reverse proxy, HTTPS on 8443)
- Frontend directly: http://localhost:3000
- OAuth service directly: http://localhost:8081
- Database (oauth): localhost:5432
- Database (history): localhost:5433

### Frontend (React/Vite)

```bash
cd front-end
npm install
npm run dev      # dev server on :5173
npm run build
npm run lint
```

### Rust services (per-crate)

```bash
cargo build                  # from individual crate dir
cargo test
cargo clippy
```

### Database migrations (Diesel)

Migrations live in `db/migrations/`. The `db` crate is the migration runner. Run via Docker using the `dev` profile:
```bash
docker compose --profile dev run --rm dev diesel migration run
```

Required env: `DATABASE_URL=postgres://diesel:dieselpass@localhost:5432/transcendance_oauth`

## Required environment variables

Set these before running (copy to `.env` or export):

```
GITHUB_ID=<your GitHub OAuth app client ID>
GITHUB_SECRET=<your GitHub OAuth app client secret>
JWT_SECRET=<random secret>
REFRESH_TOKEN_SECRET=<random secret>
SESSION_KEY=<64+ byte random string>
OAUTH_REDIRECT_URI=http://localhost:8081/auth/github/callback
FRONTEND_REDIRECT_URL=http://localhost:3000
COOKIE_SECURE=false   # set true in production
```

The compose file provides safe defaults for all except `GITHUB_ID` and `GITHUB_SECRET`.

## Architecture

This is a 42 school project implementing a multiplayer Blackjack platform as a microservices stack. All backend services are written in Rust with actix-web.

```
Browser ──► nginx (reverse-proxy :8080/:443)
              ├── /           ──► frontend (React/Vite :3000)
              ├── /auth/      ──► oauth (:8081)
              ├── /ws         ──► ws-service (:8082)
              ├── /api/game/  ──► game-engine (:8080)
              └── /api/chat/  ──► chat (:8090)
```

### Project structure

```
transcendance/
├── chat/           — Chat service (Rust/actix-web)
├── db/             — Migration runner + shared schema (Diesel)
├── front-end/      — React 19 SPA (Vite)
├── game-engine/    — Game engine (Rust/actix-web)
├── oauth/          — Authentication service (Rust/actix-web)
├── reverse-proxy/  — nginx config + TLS certs
├── ws_service/     — WebSocket gateway (Rust/actix-web)
├── docker-compose.yml
└── docker-compose.dev.yml
```

### Services

**`oauth/`** — Authentication service (port 8081)
- Local register/login with bcrypt passwords
- GitHub OAuth 2.0 (CSRF-safe with session state)
- Issues short-lived JWT access tokens (15 min) + long-lived HMAC-hashed refresh tokens (30 days) stored in PostgreSQL
- `POST /auth/introspect` validates a JWT and returns `{user_id, username}` — used by other services to authenticate requests
- Stores users and refresh tokens in `transcendance_oauth` DB via Diesel ORM

**`game-engine/`** — Game engine (port 8080)
- Manages Blackjack game tables via `manager::GameData`
- `blackjack.rs` contains game logic; `manager.rs` contains table/session management
- Persists round history to `transcendance_history` DB via Diesel ORM
- Dispatches game events to players via `ws-service`

**`chat/`** — Chat service (port 8090)
- WebSocket rooms via actix actors (`ChatSession`, `ChatHub`)
- `GET /rooms/{room_id}/ws` — WebSocket endpoint, accepts `?name=` query param or `X-Chat-User` header
- Keeps last 50 messages in-memory per room; sends history on join
- `POST /rooms/{room_id}/messages` — HTTP endpoint to post a message programmatically

**`ws_service/`** — Authenticated WebSocket gateway (port 8082)
- `GET /ws?token=<jwt>` — authenticates via oauth's introspect endpoint, then opens a user-keyed WebSocket session
- Routes messages to game-engine or chat by inspecting `kind` prefix (`game.*`, `chat.*`)
- `POST /dispatch` — internal endpoint to push an `OutboundEvent` to specific user IDs

**`front-end/`** — React 19 SPA (Vite)
- Pages: Home, Login, Register, Game, Chat, Profile, Friends, History, Statistics, Settings, Rules
- Components: Header, GameBoard, Dealer, Table, ChatPanel
- Env vars: `VITE_AUTH_URL`, `VITE_GAME_URL`, `VITE_CHAT_URL`

**`db/`** — Migration runner crate + shared schema
- Two PostgreSQL databases: `transcendance_oauth` (users, refresh_tokens) and `transcendance_history` (game history)
- Migrations managed with Diesel CLI; schema in `db/migrations/`

**`reverse-proxy/`** — nginx
- HTTP→HTTPS redirect on port 8080; TLS on 443 with self-signed certs in `reverse-proxy/certs/`
- Handles WebSocket upgrade headers for `/ws` and `/api/chat/`

### Rust crates

Each service is its own independent Rust crate with its own `Cargo.lock` (no shared workspace). Crates: `oauth`, `chat`, `game-engine`, `ws_service`, `db`.

### Auth flow

1. Frontend calls `/auth/login` or `/auth/github/login`
2. oauth service returns `{access_token, refresh_token, username, avatar_url}`
3. Frontend stores tokens; passes `access_token` as `Bearer` header or query param `?token=` to WebSocket
4. ws-service calls `POST /auth/introspect` with the token to get `user_id`
5. When access token expires, frontend calls `POST /auth/refresh` with the refresh token to rotate both tokens

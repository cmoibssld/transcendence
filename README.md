# Luigi Jack — Multiplayer Blackjack Platform

> A 42 school project (ft_transcendence) — a fully containerised, microservices-based Blackjack platform with real-time multiplayer, live chat, OAuth 2.0, and a statistics engine.

**Team:** acolonne · aguilleu · awarlamo · cabarrou · jlepany

---

## Table of Contents

1. [Team Roles](#team-roles)
2. [Tech Stack](#tech-stack)
3. [Architecture](#architecture)
4. [Database Schema](#database-schema)
5. [Features & Who Built What](#features--who-built-what)
6. [Modules & Points](#modules--points)
7. [Getting Started](#getting-started)
8. [Environment Variables](#environment-variables)
9. [Resources](#resources)

---

## Team Roles

| Login | Role |
|---|---|
| **acolonne** | Auth service (security hardening, OAuth code exchange, rate limiting, refresh tokens) · Game engine frontend integration · Statistics & friends UI · Full frontend overhaul · Internal HTTPS |
| **aguilleu** | Database layer (Diesel ORM, PostgreSQL, migrations) · Docker Compose infrastructure · OAuth DB schema · Reverse proxy initial setup |
| **awarlamo** | Game engine (Rust/actix-web, Blackjack logic, table management) · WebSocket gateway · nginx reverse proxy + TLS · Docker orchestration |
| **cabarrou** | Profile page · Friends page · Rules page |
| **jlepany** | WebSocket service refactor · Chat service migration & ws-service integration |

---

## Tech Stack

### Backend — all services in Rust

| Crate | Purpose |
|---|---|
| `actix-web 4` | HTTP server for all 5 backend services |
| `actix` + `actix-web-actors` | Actor model for WebSocket connections (ws-service, chat) |
| `diesel 2` + `r2d2` | ORM + connection pooling (oauth, stats-service) |
| `openssl 0.10` | TLS for all internal services (`bind_openssl`) |
| `jsonwebtoken 9` | JWT signing / verification (HS256) |
| `bcrypt 0.15` | Password hashing |
| `hmac` + `sha2` | Refresh token hashing |
| `reqwest 0.12` | Async HTTP client for inter-service calls |
| `tokio 1` | Async runtime |

### Frontend

| Library | Version | Purpose |
|---|---|---|
| React | 19 | UI framework |
| Vite | 8 | Build tool / dev server |
| React Router | 7 | Client-side routing |
| react-icons | 5 | Icon set |

### Infrastructure

| Tool | Role |
|---|---|
| nginx | Reverse proxy — HTTP→HTTPS redirect, TLS termination, routing, WebSocket upgrade |
| PostgreSQL 16 | Two isolated databases (`transcendance_oauth`, `transcendance_history`) |
| Docker Compose | Container orchestration — internal bridge network, no exposed backend ports |

---

## Architecture

```
Browser
  │
  ▼  :80  ──► HTTP → HTTPS redirect
nginx :443
  │
  ├── /auth/             ──► oauth         :8081  (TLS)
  ├── /ws                ──► ws-service    :8082  (TLS, WebSocket upgrade)
  ├── /api/game/history  ──► stats-service :8083  (TLS)
  ├── /api/game/stats/   ──► stats-service :8083  (TLS)
  ├── /api/game/leader/  ──► stats-service :8083  (TLS)
  ├── /api/game/         ──► game-engine   :8080  (TLS)
  ├── /api/chat/         ──► chat          :8090  (TLS, WebSocket upgrade)
  └── /                  ──► frontend      :3000  (HTTP — TLS terminates at nginx)

Internal service calls (all HTTPS, self-signed cert, verify disabled):
  game-engine  ──POST /internal/rounds──► stats-service
  game-engine  ──POST /dispatch        ──► ws-service
  ws-service   ──POST /auth/introspect ──► oauth
  chat         ──POST /dispatch        ──► ws-service
```

All containers share the `transcendance` Docker bridge network. No backend port is exposed to the host — only nginx on `80` and `443`.

### Service ports (internal)

| Service | Port | Exposed to host |
|---|---|---|
| oauth | 8081 | — |
| game-engine | 8080 | — |
| stats-service | 8083 | — |
| ws-service | 8082 | — |
| chat | 8090 | — |
| frontend | 3000 | — |
| nginx | 80 / 443 | **80**, **443** |
| db_oauth | 5432 | — |
| db_history | 5432 | — |

### Startup chain (healthcheck dependency order)

```
db_oauth (healthy)
    └── api — migration runner (healthy)
            └── oauth (healthy)
                    ├── frontend
                    ├── stats-service (healthy)
                    │       └── game-engine (healthy)
                    │               └── ws-service
                    └── reverse-proxy
```

---

## Database Schema

### `transcendance_oauth`

```
┌─────────────────────────────┐       ┌───────────────────────────────────┐
│ users                       │       │ refresh_tokens                    │
├─────────────────────────────┤       ├───────────────────────────────────┤
│ id            SERIAL  PK    │◄──┐   │ id          BIGSERIAL  PK         │
│ username      VARCHAR(50) UQ│   └───│ user_id     INT  FK → users.id    │
│ email         VARCHAR(255)UQ│       │ token_hash  TEXT  UNIQUE          │
│ password_hash TEXT          │       │ created_at  TIMESTAMPTZ           │
│ money         BIGINT  DEF 0 │       │ expires_at  TIMESTAMPTZ           │
│ avatar        TEXT          │       │ revoked_at  TIMESTAMPTZ  NULL     │
│ provider      VARCHAR(10)   │       │ user_agent  TEXT  NULL            │
│ bio           TEXT          │       │ ip_address  TEXT  NULL            │
│ updated_at    TIMESTAMPTZ   │       └───────────────────────────────────┘
└─────────────────────────────┘
         │              │
         │   ┌──────────┴───────────────────┐
         │   │ friendships                  │
         │   ├──────────────────────────────┤
         └──►│ requester_id  INT  FK→users  │
         ┌──►│ addressee_id  INT  FK→users  │
         └───│ status  ENUM(pending,        │
             │              accepted,       │
             │              declined)       │
             │ created_at  TIMESTAMPTZ      │
             └──────────────────────────────┘
```

Constraints: `no_self_friend` (requester ≠ addressee), `unique_pair`.

### `transcendance_history`

```
┌──────────────────────────────┐       ┌─────────────────────────────────────┐
│ game_rounds                  │       │ round_players                       │
├──────────────────────────────┤       ├─────────────────────────────────────┤
│ id            BIGSERIAL  PK  │◄──┐   │ id         BIGSERIAL  PK            │
│ table_id      INTEGER        │   └───│ round_id   BIGINT  FK→game_rounds   │
│ round_number  INTEGER        │       │ user_id    BIGINT                   │
│ dealer_points INTEGER        │       │ bet        BIGINT                   │
│ result_json   TEXT           │       │ outcome    ENUM(win, blackjack,     │
│ played_at     TIMESTAMPTZ    │       │             push, loss)             │
└──────────────────────────────┘       │ payout     BIGINT                   │
                                       │ played_at  TIMESTAMPTZ              │
                                       └─────────────────────────────────────┘

┌────────────────────────────────────┐
│ user_stats  (pre-aggregated)       │
├────────────────────────────────────┤
│ user_id        BIGINT  PK          │
│ games_played   INT                 │
│ wins           INT                 │
│ losses         INT                 │
│ pushes         INT                 │
│ total_wagered  BIGINT              │
│ total_won      BIGINT              │
│ updated_at     TIMESTAMPTZ         │
└────────────────────────────────────┘
```

---

## Features & Who Built What

### Authentication & User Management
| Feature | Owner |
|---|---|
| Email/password registration with bcrypt | aguilleu |
| GitHub OAuth 2.0 (CSRF-safe with session, one-time code exchange) | acolonne |
| JWT access tokens (15 min, HS256) | aguilleu / acolonne |
| HMAC-hashed refresh tokens (30 days, DB-stored, rotation) | acolonne |
| Per-IP rate limiting on `/auth/login` (5 req/min) | acolonne |
| `POST /auth/introspect` — token validation for downstream services | aguilleu |
| `GET /auth/users?username=` — user search for friend requests | acolonne |
| `GET /auth/profile` — authenticated profile endpoint | acolonne |
| Constant-time secret comparison (timing-attack prevention) | acolonne |
| Shared `X-Internal-Secret` header for inter-service authentication | acolonne |

### Blackjack Game Engine
| Feature | Owner |
|---|---|
| Blackjack logic (hit, stand, bust, push, blackjack detection) | awarlamo |
| Multi-table management with concurrent sessions | awarlamo |
| Per-round betting with coin deduction and payout (1:1 / 1.5:1) | awarlamo |
| Dealer AI (draws until ≥ 17) | awarlamo |
| Timer-based sit-out (skip round without auto-bet) | acolonne |
| Fire-and-forget history recording to stats-service | acolonne |
| Real-time event dispatch to players via ws-service | awarlamo |

### WebSocket Gateway (`ws-service`)
| Feature | Owner |
|---|---|
| JWT-authenticated WebSocket endpoint (`GET /ws?token=`) | awarlamo |
| Message routing by `kind` prefix (`game.*`, `chat.*`) | awarlamo |
| `POST /dispatch` — internal push to specific user sessions | awarlamo |
| Refactor to actix-web-actors with TLS | jlepany |

### Chat
| Feature | Owner |
|---|---|
| WebSocket rooms with actix actors (`ChatSession`, `ChatHub`) | awarlamo |
| Last-50-message in-memory history per room, sent on join | awarlamo |
| Per-table embedded chat column in game board | acolonne |
| HTTP `POST /rooms/{id}/messages` for programmatic posting | awarlamo |
| Chat service migration to ws-service integration | jlepany |

### Statistics Service
| Feature | Owner |
|---|---|
| `POST /internal/rounds` — round recording (secret-authenticated) | acolonne |
| `GET /history` — paginated match history | acolonne |
| `GET /stats/me` and `GET /stats/friend` — personal stats | acolonne |
| `GET /leaderboard/wins`, `/wagered`, `/gains` | acolonne |
| `user_stats` pre-aggregated totals, updated per round | acolonne |

### Infrastructure & DevOps
| Feature | Owner |
|---|---|
| Docker Compose — full stack orchestration, healthcheck chain | aguilleu / awarlamo |
| nginx reverse proxy — routing, TLS termination, WebSocket upgrade | awarlamo |
| Internal HTTPS (all backend services use `bind_openssl`) | acolonne |
| No exposed backend ports — all traffic through nginx | acolonne |
| Self-signed TLS certificates for development | awarlamo |
| PostgreSQL migrations with Diesel CLI | aguilleu |
| Stats-service split from game-engine (microservices refactor) | acolonne |

### Frontend (React 19 / Vite)
| Feature | Owner |
|---|---|
| Dark glassmorphism design system (CSS variables, Space Grotesk) | acolonne |
| Auth-aware header with avatar and logout | acolonne |
| Home, Login, Register pages | acolonne |
| Game board (cards, dealer, bet UI, timer countdown, inline chat) | acolonne |
| Profile page (avatar, bio, stats display) | cabarrou / acolonne |
| Friends page (user search, add/accept friends) | cabarrou |
| History page (match log with outcomes) | acolonne |
| Statistics page (personal stats, leaderboard) | acolonne |
| Settings page | acolonne |
| Rules page | cabarrou |
| Privacy Policy & Terms of Service pages + footer | acolonne |
| GitHub OAuth callback flow (`/auth/callback`) | acolonne |
| JWT refresh token rotation (automatic client-side) | acolonne |

---

## Modules & Points

Each **major module** = 1 point, each **minor module** = 0.5 point. A minimum of 7 major modules (or equivalent) is required.

| # | Module | Type | Points | Owner(s) |
|---|---|---|---|---|
| 1 | **Backend Framework** — all services built in Rust/actix-web | Major | 1.0 | aguilleu / awarlamo |
| 2 | **Standard User Management** — registration, login, profile, avatar, friends | Major | 1.0 | acolonne / cabarrou |
| 3 | **OAuth 2.0 Authentication** — GitHub OAuth with CSRF protection and secure code exchange | Major | 1.0 | acolonne |
| 4 | **Remote Players** — real-time multiplayer Blackjack via WebSocket | Major | 1.0 | awarlamo |
| 5 | **Live Chat** — rooms, 50-message history, embedded in-game panel | Major | 1.0 | awarlamo / jlepany |
| 6 | **Microservices Architecture** — 5 independent backend services, internal HTTPS | Major | 1.0 | all |
| 7 | **Server-Side Game Logic** — Blackjack engine fully server-side with validated outcomes | Major | 1.0 | awarlamo / acolonne |
| 8 | **User and Game Stats Dashboards** — stats, leaderboards, match history | Minor | 0.5 | acolonne |
| 9 | **Expanding Browser Compatibility** — responsive layout, progressive enhancement | Minor | 0.5 | acolonne |

**Total: 7 major + 2 minor = 8 points**

---

## Getting Started

### Prerequisites

- Docker + Docker Compose v2
- A GitHub OAuth App ([create one](https://github.com/settings/developers))
  - Homepage URL: `https://localhost:8443`
  - Authorization callback URL: `https://localhost:8443/auth/github/callback`

### Quick start

```bash
# 1. Copy and fill in required secrets
cp .env.example .env
# Edit .env — set GITHUB_ID, GITHUB_SECRET, and regenerate the random secrets

# 2. Build and start the full stack
docker compose up --build

# 3. Open the app (accept the self-signed cert warning in your browser)
open https://localhost:8443
```

### Development (live reload)

```bash
docker compose -f docker-compose.yml -f docker-compose.dev.yml up
```

This mounts source directories and enables live reload for frontend, game-engine, and chat.

### Running services individually

```bash
# Frontend
cd front-end
npm install
npm run dev        # Vite dev server on :5173
npm run lint

# Any Rust service
cd oauth           # or game-engine, stats-service, ws_service, chat
cargo build
cargo test
cargo clippy
```

### Database migrations

Migrations run automatically at startup via the `api` container. To run manually:

```bash
# OAuth database
docker compose --profile dev run --rm dev diesel migration run

# Stats/history database
cd stats-service
DATABASE_URL=postgres://diesel:dieselpass@localhost:5433/transcendance_history \
  diesel migration run
```

---

## Environment Variables

Copy `.env.example` to `.env` and fill in the required values.  
The compose file provides safe **development defaults** for everything except `GITHUB_ID` and `GITHUB_SECRET`.

| Variable | Required | Description |
|---|---|---|
| `GITHUB_ID` | **yes** | GitHub OAuth App client ID |
| `GITHUB_SECRET` | **yes** | GitHub OAuth App client secret |
| `JWT_SECRET` | yes | HMAC-SHA256 secret for JWT signing (`openssl rand -hex 32`) |
| `REFRESH_TOKEN_SECRET` | yes | Secret for hashing refresh tokens (`openssl rand -hex 32`) |
| `SESSION_KEY` | yes | ≥64-byte key for CSRF session cookie (`openssl rand -hex 64`) |
| `INTERNAL_SECRET` | yes | Shared secret for inter-service calls (`openssl rand -hex 32`) |
| `OAUTH_REDIRECT_URI` | yes | GitHub OAuth callback, e.g. `https://localhost:8443/auth/github/callback` |
| `FRONTEND_REDIRECT_URL` | yes | Post-OAuth redirect, e.g. `https://localhost:8443/auth/callback` |
| `COOKIE_SECURE` | no | `true` in production (HTTPS only), `false` in dev |

---

## Resources

- [The Rust Programming Language](https://doc.rust-lang.org/book/) — primary Rust reference
- [The Rocket Programming Guide](https://rocket.rs/guide/v0.5/) — early framework exploration
- [Actix Web Documentation](https://actix.rs/docs/) — framework used for all backend services
- [Diesel ORM Guide](https://diesel.rs/guides/) — database access and migrations
- [Are we web yet?](https://www.arewewebyet.org/) — Rust web ecosystem overview
- [Simple Reverse Proxy](https://github.com/MarmadileManteater/SimpleReverseProxy) — nginx gateway inspiration

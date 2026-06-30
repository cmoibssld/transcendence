*This project has been created as part of the 42 curriculum by acolonne, aguilleu, cabarrou, jlepany.*

# Luigi Jack — Multiplayer Blackjack Platform

## Description

The goal of this project is to create a fully functional, containerised web application as part of the 42 *ft_transcendence* curriculum. 

Luigi Jack is a microservices-based Blackjack platform that allows users to play real-time multiplayer games, interact via live chat, securely authenticate using OAuth 2.0, and track their performance through a comprehensive statistics engine. It demonstrates advanced technical skills, including a Rust backend, WebSocket synchronization, strict server-side game logic validation, and a modern React frontend.

**Team:** acolonne · aguilleu · cabarrou · jlepany

---

## Table of Contents

1. [Description](#description)
2. [Team Information & Roles](#team-information--roles)
3. [Project Management](#project-management)
4. [Technical Stack](#technical-stack)
5. [Architecture](#architecture)
6. [Database Schema](#database-schema)
7. [Individual Contributions & Features](#individual-contributions--features)
8. [Modules & Points](#modules--points)
9. [Instructions](#instructions)
10. [Environment Variables](#environment-variables)
11. [Resources](#resources)

---

## Team Information & Roles

According to the project requirements, our team was structured with the following agile roles and technical responsibilities:

| Login | Project Role | Technical Responsibilities |
|---|---|---|
| **acolonne** | **Product Owner (PO) / Developer** | Auth service (security hardening, OAuth code exchange, rate limiting, refresh tokens) · Chatservice |
| **aguilleu** | **Scrum Manager (PM) / Developer** | Database layer (PostgreSQL with ORM) · Game Statistics · OAuth DB schema |
| **jlepany** | **Architect (Tech Lead) / Developer** | WebSocket service · Game engine (Rust/actix-web, Blackjack logic, table management) |
| **cabarrou** | **Developer** | Frontend and assets · Friends page · Rules page |

---

## Project Management

* **Task Distribution & Tracking:** Use of Focalboard (Trello-like tool to organize work) and git branch.
* **Communication Channels:** Weekly meet-ups and continous Discord communication.
* **Code Review & Workflow:** Work on two-person team to overcome technical obstacles. Code review by one other team member with explaination.

---

## Technical Stack

### Backend — all services in Rust

| Crate | Purpose |
|---|---|
| `actix-web 4` | HTTP server for all 5 backend services + actix web actor to handle websockets |
| `diesel 2`| ORM for two PostgreSql db (oauth, stats-service) |
| `jsonwebtoken 9` | JWT signing / verification (HS256) |
| `bcrypt 0.15` | Password hashing |
| `openssl 0.10` | SSL encryption for HTTP communication |


### Frontend

| Library | Version | Purpose |
|---|---|---|
| React | 19 | UI framework |
| Tailwind | 4 | CSS framework |
| Vite | 8 | Build tool / dev server |
| React Router | 7 | Client-side routing |
| react-icons | 5 | Icon set |

### Infrastructure

| Tool | Role |
|---|---|
| nginx | Reverse proxy — HTTP→HTTPS redirect, TLS termination, routing, WebSocket upgrade |
| PostgreSQL 16 | Two isolated databases (`transcendance_oauth`, `transcendance_history`) |
| Docker Compose | Container orchestration — no exposed backend ports |

---

## Architecture

```text
Browser
  │
  ▼  :80  ──► HTTP → HTTPS redirect
nginx :443
  │
  ├── /auth/             ──► oauth         :8081  (TLS)
  ├── /ws/               ──► ws-service    :8082  (TLS, WebSocket upgrade)
  ├── /db/               ──► ORM           :3443  (TLS)
  ├── /game/             ──► game-engine   :8080  (TLS)
  ├── /chat/             ──► chat          :8090  (TLS, WebSocket upgrade)
  └── /                  ──► frontend      :3000  (HTTP — TLS terminates at nginx)
````
All containers share the transcendance Docker bridge network. No backend port is exposed to the host — only nginx on 80 and 443.

### Service ports (internal)

| Service | Port | Exposed to host |
| :--- | :--- | :--- |
| oauth | 8081 | — |
| game-engine | 8080 | — |
| stats-service | 8083 | — |
| ws-service | 8082 | — |
| chat | 8090 | — |
| frontend | 3000 | — |
| nginx | 80 / 443 | 80, 443 |
| db_oauth | 5432 | — |
| db_history | 5432 | — |

---

## Database Schema

### transcendance_oauth

```plaintext
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
````
Constraints: no_self_friend (requester ≠ addressee), unique_pair.

### transcendance_history

```text
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

## Individual Contributions & Features

### Authentication & User Management (acolonne, aguilleu)
* **Email/password registration** with bcrypt (acolonne)
* **GitHub OAuth 2.0** (CSRF-safe with session, one-time code exchange) (acolonne)
* **JWT access & refresh tokens** (HMAC-hashed, rotation) (acolonne / aguilleu)
* **Per-IP rate limiting** & Constant-time secret comparison (acolonne)
* **POST /auth/introspect** for downstream services validation (aguilleu)
* **User profile, friends search endpoints** (aguilleu)
* **Challenges faced:** Managing secure token rotation without exposing secrets to the frontend required implementing a strict HttpOnly cookie strategy and one-time code exchange.

### Blackjack Game Engine (jlepany)
* **Blackjack logic, dealer AI, multi-table management** (jlepany)
* **Per-round betting** with coin deduction and payout (jlepany)
* **Timer-based sit-out** & fire-and-forget history recording (jlepany)
* **Real-time event dispatch** via ws-service (jlepany)
* **Challenges faced:** Ensuring atomic coin deduction to prevent race conditions during concurrent bets required implementing a strict lock mechanism and rollback workflow.

### WebSocket Gateway & Chat (jlepany, acolonne)
* **JWT-authenticated WebSocket endpoint & routing** (acolonne)
* **Actix-web-actors refactor** with TLS (jlepany)
* **Chat service migration** & ws-service integration (jlepany)
* **In-memory chat history** per room & embedded in-game panel (acolonne)
* **Challenges faced:** Decoupling the WebSocket gateway from the chat logic required building a reliable internal message dispatcher across microservices.]

### Statistics Service (cabarrou)
* **Round recording** & paginated match history (cabarrou)
* **Personal stats & Leaderboards** (`user_stats` pre-aggregated totals) (acolonne)

### Infrastructure & DevOps (aguilleu, jlepany)
* **Docker Compose** & nginx reverse proxy (aguilleu, jlepany)
* **Internal HTTPS** & restricted backend ports (jlepany)
* **PostgreSQL migrations** with Diesel CLI (aguilleu)

### Frontend - React 19 / Vite (cabarrou, acolonne)
* **Dark glassmorphism design system** & Game board (cabarrou)
* **Profile, Friends, and Rules pages** (cabarrou)
* **History, Statistics, Settings, Privacy Policy & TOS pages** (cabarrou)
* **GitHub OAuth callback flow** & JWT refresh handling (acolonne)

---

## Modules & Points
Each major module = 2 points, each minor module = 1 point. A minimum of 14 points is required.

| # | Module Type | Points | Owner(s) | Implementation Summary |
| :-: | --- | :-: | --- | --- |
| **1** | Backend and Frontend Framework (Major) | 2 | Built entirely with Rust and actix-web for high performance. React and Tailwind for the frontend |
| **2** | Real-time features using WebSockets| 2 | WebSocket rooms, blackjack game using real-time WebSockets and livechat |
| **3** | User interacting with each other (Major) | 2 | Live chat + profile user information + friends system |
| **4** | ORM for the dtabase (Minor) | 1 | Diesel ORM database
| **5** | Standard User Management (Major) | 2 | Full auth flow, avatars, profile, friends system. |
| **6** | User and Game Stats Dashboards (Minor) | 1 | Comprehensive stats, leaderboards, and detailed match history pages. |
| **7** | OAuth 2.0 Authentication (Minor) | 1 | Integrated GitHub OAuth with CSRF protection and secure code exchange. |
| **8** | Complete web-based game where users can play against each other (Major) | 2 | Complete multiplayer Blackjack where players play against each other.|
| **9** | Remote players (Major) | 2 | Enable two players on separate computers to play the same game in real-time
| **10** | Multiplayer game (Major) | 2 | 3 players can play at the same time on the same table
| **11** | Backend as microservices. (Major) | 2 | Independent backend services communicating over internal HTTPS.

**Total:** 8 major × 2 + 3 minor × 1 = **19 points**


## Instructions

### Prerequisites
* Docker + Docker Compose v2
* A GitHub OAuth App ([create one](https://github.com/settings/developers))
  * **Homepage URL:** `https://url:8443`
  * **Authorization callback URL:** `https://url:8443/auth/github/callback`

### Execution

    # 1. Copy and fill in required secrets
    cp .env.example .env
    # Edit .env — set GITHUB_ID, GITHUB_SECRET, and regenerate the random secrets

    # 2. Build and start the full stack
    docker compose up --build

    # 3. Open the app (accept the self-signed cert warning in your browser)
    open https://localhost:8443

### Compilation and Development (Live Reload)

    docker compose -f docker-compose.yml up

This mounts source directories and enables live reload for frontend, game-engine, and chat.


### Database installations & migrations
Migrations run automatically at startup via the api container.


## Environment Variables

Copy `.env.example` to `.env` and fill in the required values.  
The compose file provides safe development defaults for everything except `GITHUB_ID` and `GITHUB_SECRET`.

| Variable | Required | Description |
| --- | :-: | --- |
| `GITHUB_ID` | **yes** | GitHub OAuth App client ID |
| `GITHUB_SECRET` | **yes** | GitHub OAuth App client secret |
| `JWT_SECRET` | **yes** | HMAC-SHA256 secret for JWT signing (`openssl rand -hex 32`) |
| `REFRESH_TOKEN_SECRET` | **yes** | Secret for hashing refresh tokens (`openssl rand -hex 32`) |
| `SESSION_KEY` | **yes** | ≥64-byte key for CSRF session cookie (`openssl rand -hex 64`) |
| `INTERNAL_SECRET` | **yes** | Shared secret for inter-service calls (`openssl rand -hex 32`) |
| `OAUTH_REDIRECT_URI` | **yes** | GitHub OAuth callback, e.g. `https://url:8443/auth/github/callback` |
| `FRONTEND_REDIRECT_URL` | **yes** | Post-OAuth redirect, e.g. `https://url:8443/auth/callback` |
| `COOKIE_SECURE` | no | `true` in production (HTTPS only), `false` in dev |

---

## Resources

### Artificial Intelligence Usage
As required by the curriculum guidelines, AI was utilized strategically to reduce repetitive tasks and assist with architectural brainstorming. No generated code was integrated without full comprehension and peer review.
* **Architectural Planning:** Brainstorm patterns for microservices communication in Rust (HTTP vs. WebSockets).
* **Boilerplate & CSS:** Generating repetitive HTML/React component structures and standard Tailwind CSS classes in the frontend.
* **Debugging Assistance:** CHelp understand and debug specific Rust borrowing, ownership, and lifetime issues within the game engine logic.

### Classic References & Documentation
* [The Rust Programming Language](https://doc.rust-lang.org/book/) — Primary documentation for Rust concepts.
* [Actix Web Documentation](https://actix.rs/docs/) — Documentation for the framework used for all backend microservices.
* [Diesel ORM Guide](https://diesel.rs/guides/) — Used for database schema design and migrations in PostgreSQL.
* [Are we web yet?](https://www.arewewebyet.org/) — Reference for the Rust web ecosystem and crate selection.
* [Simple Reverse Proxy](https://nginx.org/en/docs/http/ngx_http_proxy_module.html) — Inspiration and reference for configuring the Nginx gateway and TLS termination.
* [React Router Documentation](https://reactrouter.com/) — For implementing client-side routing in the frontend.
This project has been created as part of the 42 curriculum by jlepany, aguilleu, acolonne, cabarrou

## Objectif

Afficher les données joueur depuis PostgreSQL dès le début de partie (avatar, coins, username).

## Règles de communication

- Backends entre eux: appels via API REST
- Front ↔ Back: payloads JSON
- Backend API: exposée en HTTPS

## Stack

- Front: React + Vite (`capu/React/Transcendance`)
- API: Node.js + Express + PostgreSQL (`capu/api`)
- DB: PostgreSQL via Docker Compose (`capu/docker-compose.yml`)

## Démarrage rapide

```bash
cd capu
docker-compose up -d db
```

```bash
cd capu/api
cp .env.example .env
npm install
npm run dev
```

```bash
cd capu/React/Transcendance
cp .env.example .env
npm install
npm run dev
```

## API disponible

- `GET /health`
- `GET /api/players`
- `GET /api/players/:id`
- `PATCH /api/players/:id` (`money`, `avatar`)

Toutes les routes REST répondent en JSON (`res.json(...)`).

## Vérification rapide

```bash
curl -k https://localhost:3001/health
curl -k https://localhost:3001/api/players/1
```

`-k` est requis avec le certificat local auto-signé de dev.

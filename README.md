This project has been created as part of the 42 curriculum by acolonne, aguilleu, awarlamo, cabarrou, jlepany

### Description

#### Backend as microservices
The backend is composed of multiple microservices, each one running in a separate container. The main services are:
- oauth
- game-engine
- reverse-proxy
- chat

The reverse proxy is the public entry point for the application. It serves the frontend on `http://localhost:8080`, and forwards websocket traffic to the game engine and chat services behind it. The direct frontend service is still available on `http://localhost:3000`.

Chat websocket traffic also goes through the same proxy on `http://localhost:8080` under `/chat/ws/<room>`.

The top-level `game/` folder is kept as a legacy reference copy. The active game backend used by the compose stack lives under `docker/game-engine/`.

### Instructions

To start the project, run the following command in the root of the project:
```bash
docker compose up --build
```

Then open http://localhost:8080 for the application. The direct frontend server remains available on http://localhost:3000 for development.

For a live-reload development flow, use the override file:
```bash
docker compose -f docker-compose.yml -f docker-compose.dev.yml up
```

That keeps the frontend, game engine, and chat services running from mounted source directories so code changes apply without rebuilding images.


### Resources

- The Rust Programming Language, https://doc.rust-lang.org/book/
Main source of learning to master Rust language and overcome difficulty.

- The Rocket Programming Guide, https://rocket.rs/guide/v0.5/
An easy framework for a first version of the project

- Actix Web Documentation, https://actix.rs/docs/
Backend framework guide to put in place most of the features

- Are we web yet?, https://www.arewewebyet.org/
A most useful website to choose between Rust web framework and get examples and help for web developpement in Rust

- To create the reverse-proxy to use as a gateway for the multi-container backend, we used this repo for inspiration: https://github.com/MarmadileManteater/SimpleReverseProxy

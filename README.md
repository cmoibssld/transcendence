This project has been created as part of the 42 curriculum by acolonne, aguilleu, awarlamo, cabarrou, jlepany

### Description

#### Backend as microservices
The backend is composed of multiple microservices, each one running in a separate container. The microservices are:
- oauth
- game (session + engine)
- chat
- user

The entry point of the backend is a proxy/load balancer that receive request given from the frontend and forward them to the right microservice. The auth service is handled at the edge of the load balance. The token send is verified for each request.  
Livechat and game work through websocket connexion. The websocket connexion is handled by the gateway routes WS connection to the concerned microservice. The gateway supports WS pass-through.

TODO: that load balancer between the service that must support web-socket pass-through;

### Instructions

To allow HTTPS connexion inside the backend, first create a file for docker secrets and generate a self-signed certificate and a private key with the following commands:

```bash
mkdir -p docker/secrets
openssl req -x509 -newkey rsa:4096 -keyout docker/secrets/key.pem -out docker/secrets/cert.pem -days 365 -nodes
```
The certificate will be references as self-signed which might cause some warnings. To avoid this it could be possible to use something like [mkcert](https://github.com/FiloSottile/mkcert).

Then, to start the project, run the following command in the root of the project:
```bash
docker compose up --build
```

and then connect to http://localhost:8080;


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

- To understand how to optimize Docker images for Rust binaries: https://oneuptime.com/blog/post/2026-01-07-rust-minimal-docker-images/view

- Speeding up compile time with Cargo: https://users.rust-lang.org/t/is-possible-to-speedup-cargo-build-command/104416/3

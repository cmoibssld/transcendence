This project has been created as part of the 42 curriculum by jlepany, aguilleu, acolonne, cabarrou

## Données stockées pour un joueur/client

La table `users` (voir `migrations/2026-04-16-113313-0000_init_users/up.sql`) stocke:

- `id` (`SERIAL`, clé primaire)
- `username` (`VARCHAR(50)`, unique, obligatoire)
- `email` (`VARCHAR(255)`, unique, obligatoire)
- `password_hash` (`TEXT`, obligatoire)
- `money` (`BIGINT`, obligatoire, défaut `0`)

Le mapping Rust est défini dans `src/lib.rs` via la struct `User`.

## Opérations Diesel disponibles

Dans `src/lib.rs`, les fonctions suivantes permettent de manipuler la DB:

- `create_user` (upsert sur `email`)
- `list_users`
- `get_user_by_id`
- `update_user`
- `delete_user`

Le binaire `src/bin/seed.rs` utilise désormais un pool de connexions.

## Concurrence (pool Diesel)

Le pool est construit avec `diesel::r2d2` via `establish_pool`.

Cette approche permet:

- plusieurs lectures/écritures simultanées,
- de limiter les connexions ouvertes,
- d’éviter le partage dangereux d’une connexion unique entre threads.

## Diesel avec Actix-Web (recommandation officielle)

D’après la doc Actix (`https://actix.rs/docs/databases`):

- Diesel est synchrone,
- il faut exécuter les accès DB dans `web::block`,
- injecter le pool dans l’état applicatif avec `web::Data<DbPool>`.

Exemple minimal:

```rust
use actix_web::{error, web, HttpResponse};
use diesel_demo::{create_user, DbPool};

async fn create_user_handler(pool: web::Data<DbPool>) -> Result<HttpResponse, actix_web::Error> {
	let username = "player42".to_string();
	let email = "player42@example.com".to_string();
	let password_hash = "hashed_pw".to_string();

	let pool = pool.clone();
	let user = web::block(move || {
		let mut conn = pool
			.get()
			.map_err(|e| e.to_string())?;
		create_user(&mut conn, &username, &email, &password_hash).map_err(|e| e.to_string())
	})
	.await
	.map_err(error::ErrorInternalServerError)?
	.map_err(error::ErrorInternalServerError)?;

	Ok(HttpResponse::Ok().json(user.id))
}
```

## Commandes utiles Diesel

```bash
diesel migration run
cargo run --bin seed
```

## Colonnes finales (et rien d'autre)

La table `users` ne contient que ces colonnes:

- `id`
- `username`
- `email`
- `password_hash`
- `money`
- `avatar`

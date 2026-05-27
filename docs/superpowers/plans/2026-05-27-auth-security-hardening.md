# Auth Security Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix three critical security gaps in the auth service before adding the friends system: tokens exposed in OAuth redirect URLs, no authentication on the internal coins endpoint, and no rate limiting on login.

**Architecture:** All changes are confined to `oauth/src/main.rs`, `game-engine/src/main.rs`, and `docker-compose.yml`. The OAuth callback switches from passing tokens as query params to a server-side one-time code that the SPA exchanges via a second request. The internal coins endpoint gains a shared secret header check. A fourth task adds a `GET /auth/users` endpoint needed by the upcoming friends system.

**Tech Stack:** Rust/actix-web 4, Diesel/PostgreSQL, React 19/Vite. No new dependencies required.

---

## File Map

| File | Change |
|------|--------|
| `oauth/src/main.rs` | Add `CodeStore` type + `exchange_code` handler; modify `github_callback`; add secret check in `update_coins`; add `lookup_user` handler; add in-memory login rate limiter; register new app data and routes |
| `game-engine/src/main.rs` | Add `internal_secret` to `ServiceConfig`; thread secret through `deduct_coins`, `refund_coins`, and the spawned coin-update closure |
| `docker-compose.yml` | Add `INTERNAL_SECRET` env var to `oauth` and `game-engine` service blocks |
| `front-end/src/App.jsx` | Replace query-param token reading with async code exchange in `OAuthCallback` |

---

## Task 1: OAuth callback — one-time code exchange

**Files:**
- Modify: `oauth/src/main.rs`
- Modify: `front-end/src/App.jsx`

The current flow passes `access_token` and `refresh_token` as URL query params, which ends up in browser history, nginx logs, and Referer headers. The fix: `github_callback` generates a random 32-char code, stores `(AuthResponse, Instant)` in an in-memory map, and redirects with only `?code=XXX`. The SPA then calls `GET /auth/exchange?code=XXX` to receive the tokens over the CORS-controlled API.

- [ ] **Step 1: Add `#[derive(Clone)]` to `AuthResponse`**

In `oauth/src/main.rs`, change line ~51:

```rust
#[derive(Serialize, Clone)]
struct AuthResponse {
    access_token: String,
    refresh_token: String,
    username: String,
    avatar_url: Option<String>,
}
```

- [ ] **Step 2: Add `CodeStore` type alias and `ExchangeQuery` struct**

Add after the existing type aliases near the top of `oauth/src/main.rs` (after `type HmacSha256 = ...`):

```rust
use std::collections::HashMap;
use std::time::Instant;

type CodeStore = Mutex<HashMap<String, (AuthResponse, Instant)>>;

#[derive(Deserialize)]
struct ExchangeQuery {
    code: String,
}
```

`std::sync::Mutex` is already available via `use std::sync::Mutex;` — add that import if not present (check the existing imports).

- [ ] **Step 3: Write the failing test for `exchange_code` (invalid code)**

Add a `#[cfg(test)]` module at the bottom of `oauth/src/main.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App};
    use std::sync::Mutex;
    use std::collections::HashMap;

    #[actix_web::test]
    async fn test_exchange_code_invalid() {
        let store: web::Data<CodeStore> = web::Data::new(
            Mutex::new(HashMap::new())
        );
        let app = test::init_service(
            App::new()
                .app_data(store.clone())
                .route("/auth/exchange", web::get().to(exchange_code))
        ).await;

        let req = test::TestRequest::get()
            .uri("/auth/exchange?code=doesnotexist")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[actix_web::test]
    async fn test_exchange_code_valid_consumed() {
        let store: web::Data<CodeStore> = web::Data::new(
            Mutex::new(HashMap::new())
        );
        // Pre-seed one code
        {
            let mut map = store.lock().unwrap();
            map.insert("testcode123".to_string(), (
                AuthResponse {
                    access_token: "tok".into(),
                    refresh_token: "ref".into(),
                    username: "alice".into(),
                    avatar_url: None,
                },
                Instant::now(),
            ));
        }
        let app = test::init_service(
            App::new()
                .app_data(store.clone())
                .route("/auth/exchange", web::get().to(exchange_code))
        ).await;

        // First call: 200
        let req = test::TestRequest::get()
            .uri("/auth/exchange?code=testcode123")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        // Second call with same code: 400 (consumed)
        let req = test::TestRequest::get()
            .uri("/auth/exchange?code=testcode123")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }
}
```

- [ ] **Step 4: Run tests to see them fail**

```bash
cd /home/elark/transcendance/oauth
cargo test test_exchange 2>&1 | tail -20
```

Expected: compile error — `exchange_code` not defined yet.

- [ ] **Step 5: Implement `exchange_code` handler**

Add this function in `oauth/src/main.rs`, before `main()`:

```rust
async fn exchange_code(
    query: web::Query<ExchangeQuery>,
    store: web::Data<CodeStore>,
) -> impl Responder {
    let code = query.code.trim().to_string();
    if code.is_empty() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "missing code"}));
    }

    let mut map = store.lock().unwrap();
    match map.get(&code) {
        Some((_, created_at)) if created_at.elapsed() > std::time::Duration::from_secs(60) => {
            map.remove(&code);
            HttpResponse::Gone().json(serde_json::json!({"error": "code expired"}))
        }
        Some(_) => {
            let (resp, _) = map.remove(&code).unwrap();
            HttpResponse::Ok().json(resp)
        }
        None => HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "invalid code"})),
    }
}
```

- [ ] **Step 6: Run tests to see them pass**

```bash
cd /home/elark/transcendance/oauth
cargo test test_exchange 2>&1 | tail -10
```

Expected:
```
test tests::test_exchange_code_invalid ... ok
test tests::test_exchange_code_valid_consumed ... ok
```

- [ ] **Step 7: Modify `github_callback` to issue a one-time code instead of tokens in the URL**

The function signature gains `store: web::Data<CodeStore>` as a new parameter. Change the end of `github_callback` (replace the `redirect_url` format! block):

Current (lines ~430–447):
```rust
let response_data = issue_tokens(pool, user.id, user.username.clone(), Some(avatar), &req).await?;

let frontend_redirect = env::var("FRONTEND_REDIRECT_URL")
    .unwrap_or_else(|_| "http://localhost:3000".to_string());

// Pass tokens as query params so the SPA can store them in localStorage
let redirect_url = format!(
    "{}?access_token={}&refresh_token={}&username={}&avatar_url={}",
    frontend_redirect,
    urlencoding::encode(&response_data.access_token),
    urlencoding::encode(&response_data.refresh_token),
    urlencoding::encode(&user.username),
    urlencoding::encode(response_data.avatar_url.as_deref().unwrap_or("")),
);

Ok(HttpResponse::Found()
    .append_header(("Location", redirect_url))
    .finish())
```

Replace with:
```rust
let response_data = issue_tokens(pool, user.id, user.username.clone(), Some(avatar), &req).await?;

let frontend_redirect = env::var("FRONTEND_REDIRECT_URL")
    .unwrap_or_else(|_| "http://localhost:3000".to_string());

let code: String = rand::thread_rng()
    .sample_iter(&Alphanumeric)
    .take(32)
    .map(char::from)
    .collect();
store.lock().unwrap().insert(code.clone(), (response_data, std::time::Instant::now()));

let redirect_url = format!("{}?code={}", frontend_redirect, urlencoding::encode(&code));

Ok(HttpResponse::Found()
    .append_header(("Location", redirect_url))
    .finish())
```

Also update the function signature to add `store: web::Data<CodeStore>` after `session: Session`:

```rust
async fn github_callback(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    session: Session,
    store: web::Data<CodeStore>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> Result<HttpResponse> {
```

- [ ] **Step 8: Register `CodeStore` and add `/auth/exchange` route in `main()`**

In `main()`, add `CodeStore` data and the new route. Find the `let session_key = ...` block and add after it:

```rust
let code_store: web::Data<CodeStore> = web::Data::new(Mutex::new(HashMap::new()));
```

Then in `HttpServer::new(move || { App::new()... })`, add `.app_data(code_store.clone())` alongside the other `app_data` calls, and add the route:

```rust
.route("/auth/exchange", web::get().to(exchange_code))
```

Place it near the other `/auth/*` routes.

- [ ] **Step 9: Update `OAuthCallback` in `front-end/src/App.jsx`**

Replace the entire `OAuthCallback` function (lines 19–41):

```jsx
function OAuthCallback() {
  const navigate = useNavigate()
  const location = useLocation()

  useEffect(() => {
    const params = new URLSearchParams(location.search)
    const code = params.get("code")
    if (!code) return

    // Strip code from URL immediately — tokens never touch the URL
    navigate("/", { replace: true })

    const authUrl = import.meta.env.VITE_AUTH_URL ?? "http://localhost:8081"
    fetch(`${authUrl}/auth/exchange?code=${encodeURIComponent(code)}`)
      .then(r => r.json())
      .then(data => {
        if (data.access_token && data.refresh_token) {
          localStorage.setItem("access_token", data.access_token)
          localStorage.setItem("refresh_token", data.refresh_token)
          if (data.username) localStorage.setItem("username", data.username)
          if (data.avatar_url) localStorage.setItem("avatar_url", data.avatar_url)
        }
      })
      .catch(() => {})
  }, [location.search, navigate])

  return null
}
```

- [ ] **Step 10: Build and verify**

```bash
cd /home/elark/transcendance/oauth && cargo build 2>&1 | tail -20
cd /home/elark/transcendance/front-end && npm run build 2>&1 | tail -10
```

Both should succeed with no errors.

- [ ] **Step 11: Commit**

```bash
git add oauth/src/main.rs front-end/src/App.jsx
git commit -m "security: replace OAuth URL tokens with one-time code exchange"
```

---

## Task 2: Internal coins endpoint — shared secret

**Files:**
- Modify: `oauth/src/main.rs`
- Modify: `game-engine/src/main.rs`
- Modify: `docker-compose.yml`

The `POST /auth/internal/coins` endpoint has no authentication. Any container on the Docker network can call it. The fix: both services read `INTERNAL_SECRET` from the environment; game-engine sends it as an `X-Internal-Secret` header; oauth checks it before touching the DB.

- [ ] **Step 1: Add the `check_internal_secret` stub outside the test module, and the tests inside it**

First, add the stub function in `oauth/src/main.rs` **outside** the `#[cfg(test)]` block, before `update_coins`:

```rust
fn check_internal_secret(req: &HttpRequest) -> Option<HttpResponse> {
    todo!()
}
```

Then, add these tests **inside** the existing `#[cfg(test)]` module:

```rust
#[test]
fn test_internal_secret_rejects_missing_header() {
    std::env::set_var("INTERNAL_SECRET", "supersecret");
    let req = actix_web::test::TestRequest::default().to_http_request();
    assert!(check_internal_secret(&req).is_some());
    std::env::remove_var("INTERNAL_SECRET");
}

#[test]
fn test_internal_secret_rejects_wrong_value() {
    std::env::set_var("INTERNAL_SECRET", "supersecret");
    let req = actix_web::test::TestRequest::default()
        .insert_header(("X-Internal-Secret", "wrongvalue"))
        .to_http_request();
    assert!(check_internal_secret(&req).is_some());
    std::env::remove_var("INTERNAL_SECRET");
}

#[test]
fn test_internal_secret_accepts_correct_value() {
    std::env::set_var("INTERNAL_SECRET", "supersecret");
    let req = actix_web::test::TestRequest::default()
        .insert_header(("X-Internal-Secret", "supersecret"))
        .to_http_request();
    assert!(check_internal_secret(&req).is_none());
    std::env::remove_var("INTERNAL_SECRET");
}
```

Note: these are regular `#[test]` (not `#[actix_web::test]`) because they don't need a runtime.

- [ ] **Step 2: Run tests to see them fail**

```bash
cd /home/elark/transcendance/oauth
cargo test test_internal_secret 2>&1 | tail -10
```

Expected: compile error — `check_internal_secret` body is `todo!()`.

- [ ] **Step 3: Implement `check_internal_secret` and update `update_coins`**

Replace the `todo!()` placeholder with a real implementation. Add `check_internal_secret` before `update_coins` in `oauth/src/main.rs`:

```rust
fn check_internal_secret(req: &HttpRequest) -> Option<HttpResponse> {
    let secret = match env::var("INTERNAL_SECRET") {
        Ok(s) if !s.is_empty() => s,
        _ => return Some(HttpResponse::InternalServerError()
            .json(serde_json::json!({"error": "server misconfigured"}))),
    };
    let provided = req.headers()
        .get("X-Internal-Secret")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if provided != secret {
        return Some(HttpResponse::Unauthorized()
            .json(serde_json::json!({"error": "unauthorized"})));
    }
    None
}
```

Update `update_coins` to add `req: HttpRequest` as first parameter and call the check:

```rust
async fn update_coins(
    req: HttpRequest,
    pool: web::Data<DbPool>,
    body: web::Json<UpdateCoinsRequest>,
) -> impl Responder {
    if let Some(err) = check_internal_secret(&req) {
        return err;
    }

    let user_id = body.user_id;
    let delta    = body.delta;
    // ... rest of the function unchanged
```

- [ ] **Step 4: Run tests to see them pass**

```bash
cd /home/elark/transcendance/oauth
cargo test test_internal_secret 2>&1 | tail -10
```

Expected:
```
test tests::test_internal_secret_accepts_correct_value ... ok
test tests::test_internal_secret_rejects_missing_header ... ok
test tests::test_internal_secret_rejects_wrong_value ... ok
```

- [ ] **Step 5: Add `internal_secret` to `ServiceConfig` in game-engine**

In `game-engine/src/main.rs`, update the struct (lines ~21–24):

```rust
struct ServiceConfig {
    ws_service_url:   String,
    user_service_url: String,
    internal_secret:  String,
}
```

And where `ServiceConfig` is instantiated in `main()` (lines ~211–216):

```rust
let cfg = web::Data::new(ServiceConfig {
    ws_service_url:  std::env::var("WS_SERVICE_URL")
        .unwrap_or_else(|_| "http://ws-service:8082".to_string()),
    user_service_url: std::env::var("USER_SERVICE_URL")
        .unwrap_or_else(|_| "http://oauth:8081".to_string()),
    internal_secret: std::env::var("INTERNAL_SECRET")
        .unwrap_or_default(),
});
```

- [ ] **Step 6: Add secret to `deduct_coins` and `refund_coins` signatures**

Update both functions to accept `secret: &str` and add the header:

```rust
async fn deduct_coins(
    client: &reqwest::Client,
    user_url: &str,
    user_id: u64,
    amount: u64,
    secret: &str,
) -> bool {
    match client
        .post(format!("{}/auth/internal/coins", user_url))
        .header("X-Internal-Secret", secret)
        .json(&serde_json::json!({ "user_id": user_id as i32, "delta": -(amount as i64) }))
        .send()
        .await
    {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

async fn refund_coins(
    client: &reqwest::Client,
    user_url: &str,
    user_id: u64,
    amount: u64,
    secret: &str,
) {
    let _ = client
        .post(format!("{}/auth/internal/coins", user_url))
        .header("X-Internal-Secret", secret)
        .json(&serde_json::json!({ "user_id": user_id as i32, "delta": amount as i64 }))
        .send()
        .await;
}
```

- [ ] **Step 7: Update the three call sites in `ws_ingest` and the helper callers**

In `ws_ingest`, the two direct calls pass `&cfg.internal_secret`:

Line ~63 (deduct before bet):
```rust
if !deduct_coins(&client, &cfg.user_service_url, body.user_id, amount, &cfg.internal_secret).await {
```

Line ~85 (refund on rejection):
```rust
refund_coins(&client, &cfg.user_service_url, body.user_id, amount, &cfg.internal_secret).await;
```

For the spawned closure that posts coin updates (lines ~113–123), clone the secret before the loop:

```rust
for (uid, delta) in result.coin_updates {
    let c2     = client.clone();
    let uu     = cfg.user_service_url.clone();
    let secret = cfg.internal_secret.clone();
    actix_web::rt::spawn(async move {
        let _ = c2.post(format!("{}/auth/internal/coins", uu))
            .header("X-Internal-Secret", secret)
            .json(&serde_json::json!({ "user_id": uid as i32, "delta": delta }))
            .send()
            .await;
    });
}
```

- [ ] **Step 8: Add `INTERNAL_SECRET` to `docker-compose.yml`**

In the `oauth` service block (around line 105), add:
```yaml
      INTERNAL_SECRET: ${INTERNAL_SECRET:-dev-internal-secret-change-me}
```

In the `game-engine` service block (around line 153), add:
```yaml
      INTERNAL_SECRET: ${INTERNAL_SECRET:-dev-internal-secret-change-me}
```

Both services must use the **same** variable so they share the same secret.

- [ ] **Step 9: Build both services**

```bash
cd /home/elark/transcendance/oauth && cargo build 2>&1 | tail -10
cd /home/elark/transcendance/game-engine && cargo build 2>&1 | tail -10
```

Both should succeed.

- [ ] **Step 10: Commit**

```bash
git add oauth/src/main.rs game-engine/src/main.rs docker-compose.yml
git commit -m "security: authenticate internal coins endpoint with shared secret"
```

---

## Task 3: User lookup endpoint (prerequisite for friends)

**Files:**
- Modify: `oauth/src/main.rs`

A `GET /auth/users?username=foo` endpoint is needed to send friend requests by username. It returns public info only: id, username, avatar_url.

- [ ] **Step 1: Add the validation helper stub and its test**

Add a `validate_lookup_username` stub function **outside** the `#[cfg(test)]` block in `oauth/src/main.rs`, before `lookup_user`:

```rust
fn validate_lookup_username(username: &str) -> Option<HttpResponse> {
    todo!()
}
```

Then add these tests **inside** the `#[cfg(test)]` module:

```rust
#[test]
fn test_lookup_user_empty_username_rejected() {
    assert!(validate_lookup_username("").is_some());
    assert!(validate_lookup_username("   ").is_some());
}

#[test]
fn test_lookup_user_nonempty_username_passes() {
    assert!(validate_lookup_username("alice").is_none());
}
```

- [ ] **Step 2: Run tests to see them fail**

```bash
cd /home/elark/transcendance/oauth
cargo test test_lookup_user 2>&1 | tail -10
```

Expected: tests panic with "not yet implemented" (todo!() reached).

- [ ] **Step 2b: Add `UserLookupQuery` and `UserLookupResponse` structs**

Add these after the existing structs near the top of `oauth/src/main.rs`:

```rust
#[derive(Deserialize)]
struct UserLookupQuery {
    username: String,
}

#[derive(Serialize)]
struct UserLookupResponse {
    id: i32,
    username: String,
    avatar_url: Option<String>,
}
```

- [ ] **Step 3: Implement `validate_lookup_username` and `lookup_user` handler**

Replace the `todo!()` stub with the real implementation:

```rust
fn validate_lookup_username(username: &str) -> Option<HttpResponse> {
    if username.trim().is_empty() {
        return Some(HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "username required"})));
    }
    None
}
```

Then add the `lookup_user` handler before `main()`:

```rust
async fn lookup_user(
    query: web::Query<UserLookupQuery>,
    pool: web::Data<DbPool>,
) -> impl Responder {
    let username = query.username.trim().to_string();
    if let Some(err) = validate_lookup_username(&username) {
        return err;
    }

    let result = web::block(move || {
        use crate::schema::users::dsl as u;
        let mut conn = pool.get().expect("DB pool");
        u::users
            .filter(u::username.eq(&username))
            .select((u::id, u::username, u::avatar))
            .first::<(i32, String, String)>(&mut conn)
    })
    .await;

    match result {
        Ok(Ok((id, username, avatar))) => {
            let avatar_url = if avatar.is_empty() { None } else { Some(avatar) };
            HttpResponse::Ok().json(UserLookupResponse { id, username, avatar_url })
        }
        Ok(Err(diesel::result::Error::NotFound)) => {
            HttpResponse::NotFound()
                .json(serde_json::json!({"error": "user not found"}))
        }
        _ => HttpResponse::InternalServerError()
            .json(serde_json::json!({"error": "db error"})),
    }
}
```

- [ ] **Step 4: Register route in `main()`**

Add alongside the other `/auth/*` routes in `HttpServer::new`:

```rust
.route("/auth/users", web::get().to(lookup_user))
```

- [ ] **Step 5: Build and verify**

```bash
cd /home/elark/transcendance/oauth && cargo build 2>&1 | tail -10
```

Then with the stack running, test manually:

```bash
# User not found
curl -s http://localhost:8081/auth/users?username=nobody | jq
# Expected: {"error":"user not found"}

# User exists (replace with a real username from your DB)
curl -s http://localhost:8081/auth/users?username=your_username | jq
# Expected: {"id":1,"username":"your_username","avatar_url":null}
```

- [ ] **Step 6: Commit**

```bash
git add oauth/src/main.rs
git commit -m "feat(auth): add GET /auth/users?username= for friend lookup"
```

---

## Task 4: Login rate limiting

**Files:**
- Modify: `oauth/src/main.rs`

Brute-force protection: 5 attempts per IP per 60-second window. Uses an in-memory `HashMap` — no new dependency required.

- [ ] **Step 1: Add `LoginAttempts` type alias**

Add after the `CodeStore` type alias:

```rust
type LoginAttempts = Mutex<HashMap<String, (u32, Instant)>>;
```

- [ ] **Step 2: Add `check_rate_limit` stub outside the test module, then the tests inside it**

Add this stub **outside** the `#[cfg(test)]` block, before the `login` handler:

```rust
fn check_rate_limit(attempts: &web::Data<LoginAttempts>, ip: &str) -> bool {
    todo!()
}
```

Then add these tests **inside** the `#[cfg(test)]` module:

```rust
#[test]
fn test_rate_limit_allows_first_5() {
    let data: web::Data<LoginAttempts> = web::Data::new(Mutex::new(HashMap::new()));
    for _ in 0..5 {
        assert!(check_rate_limit(&data, "1.2.3.4"), "should be allowed");
    }
}

#[test]
fn test_rate_limit_blocks_6th() {
    let data: web::Data<LoginAttempts> = web::Data::new(Mutex::new(HashMap::new()));
    for _ in 0..5 {
        check_rate_limit(&data, "1.2.3.4");
    }
    assert!(!check_rate_limit(&data, "1.2.3.4"), "6th attempt should be blocked");
}

#[test]
fn test_rate_limit_independent_ips() {
    let data: web::Data<LoginAttempts> = web::Data::new(Mutex::new(HashMap::new()));
    for _ in 0..5 {
        check_rate_limit(&data, "1.1.1.1");
    }
    // Different IP should still be allowed
    assert!(check_rate_limit(&data, "2.2.2.2"), "different IP should be allowed");
}
```

- [ ] **Step 3: Run tests to see them fail**

```bash
cd /home/elark/transcendance/oauth
cargo test test_rate_limit 2>&1 | tail -10
```

Expected: compile error — `check_rate_limit` body is `todo!()`.

- [ ] **Step 4: Implement `check_rate_limit` function**

Replace the `todo!()` with the real implementation (keep the function outside the `#[cfg(test)]` module so it's usable by the handler):

Add before the `login` handler in `oauth/src/main.rs`:

```rust
fn check_rate_limit(attempts: &web::Data<LoginAttempts>, ip: &str) -> bool {
    let mut map = attempts.lock().unwrap();
    let now = Instant::now();
    let entry = map.entry(ip.to_string()).or_insert((0, now));
    if entry.1.elapsed() > std::time::Duration::from_secs(60) {
        *entry = (1, now);
        return true;
    }
    if entry.0 >= 5 {
        return false;
    }
    entry.0 += 1;
    true
}
```

Remove the `todo!()` stub from the test module (leave only the `#[test]` functions that call `check_rate_limit`).

- [ ] **Step 5: Run tests to see them pass**

```bash
cd /home/elark/transcendance/oauth
cargo test test_rate_limit 2>&1 | tail -10
```

Expected:
```
test tests::test_rate_limit_allows_first_5 ... ok
test tests::test_rate_limit_blocks_6th ... ok
test tests::test_rate_limit_independent_ips ... ok
```

- [ ] **Step 6: Wire rate limiter into `login` handler**

Add `attempts: web::Data<LoginAttempts>` to the `login` signature and add the check at the top:

```rust
async fn login(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    body: web::Json<LocalLoginRequest>,
    attempts: web::Data<LoginAttempts>,
) -> impl Responder {
    let ip = req
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("unknown")
        .to_string();

    if !check_rate_limit(&attempts, &ip) {
        return HttpResponse::TooManyRequests()
            .json(serde_json::json!({"error": "Trop de tentatives, réessayez dans une minute"}));
    }

    let login_data = body.into_inner();
    // ... rest of the function unchanged
```

- [ ] **Step 7: Register `LoginAttempts` in `main()`**

Add alongside the `code_store` data in `main()`:

```rust
let login_attempts: web::Data<LoginAttempts> = web::Data::new(Mutex::new(HashMap::new()));
```

And in `HttpServer::new(move || { App::new()... })`, add:

```rust
.app_data(login_attempts.clone())
```

- [ ] **Step 8: Build and run all tests**

```bash
cd /home/elark/transcendance/oauth
cargo test 2>&1 | tail -15
cargo build 2>&1 | tail -10
```

Expected: all tests pass, build succeeds.

- [ ] **Step 9: Commit**

```bash
git add oauth/src/main.rs
git commit -m "security: add per-IP rate limiting on /auth/login (5 req/min)"
```

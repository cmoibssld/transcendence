# Frontend Redesign — Moderne Sombre Élégant

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remplacer le design kitsch casino-Vegas par un style sombre élégant, rendre le header auth-aware, et connecter le Profile à un vrai endpoint API.

**Architecture:** Token swap CSS d'abord (impact immédiat sur tout le site), puis composants dans l'ordre : header → gameboard → home → login → register → profile. Le backend Rust reçoit d'abord un endpoint `GET /auth/profile` et un fix du redirect GitHub OAuth.

**Tech Stack:** Rust/actix-web (oauth service), React 19/Vite, CSS vanilla (pas de Tailwind), Diesel ORM, localStorage pour tokens auth.

---

## File Map

| Fichier | Action |
|---|---|
| `oauth/src/main.rs` | Modify — ajouter `ProfileResponse`, `get_profile`, route, fix redirect GitHub |
| `front-end/src/App.css` | Modify — tokens, body, boutons |
| `front-end/src/App.jsx` | Modify — OAuthCallback stocke `avatar_url` |
| `front-end/src/components/Header.jsx` | Rewrite — auth-aware |
| `front-end/src/styles/Header.css` | Rewrite |
| `front-end/src/components/GameBoard.jsx` | Modify — DealerAvatar, remove dealer.png |
| `front-end/src/styles/GameBoard.css` | Modify — arena, sidebar, boutons, outcome |
| `front-end/src/components/Dealer.jsx` | Delete |
| `front-end/src/components/Table.jsx` | Delete |
| `front-end/src/styles/Dealer.css` | Delete |
| `front-end/src/styles/Table.css` | Delete |
| `front-end/src/styles/Home.css` | Modify — overlay, titre |
| `front-end/src/pages/Login.jsx` | Modify — stocker avatar_url |
| `front-end/src/styles/Login.css` | Rewrite |
| `front-end/src/pages/Register.jsx` | Modify — supprimer login-kicker |
| `front-end/src/pages/Profile.jsx` | Rewrite — appel GET /auth/profile |
| `front-end/src/styles/Profile.css` | Rewrite |
| `front-end/src/assets/Aloun.jpg` | Delete |
| `front-end/src/assets/Alexis.jpg` | Delete |
| `front-end/src/assets/Adrien.jpg` | Delete |
| `front-end/src/assets/JB.jpg` | Delete |

---

## Task 1 — `GET /auth/profile` endpoint (Rust)

**Files:**
- Modify: `oauth/src/main.rs`

- [ ] **Step 1 : Ajouter `ProfileResponse` struct**

Juste après la struct `IntrospectResponse` (ligne ~62), ajouter :

```rust
#[derive(Serialize)]
struct ProfileResponse {
    username: String,
    avatar_url: Option<String>,
    coins: i64,
}
```

- [ ] **Step 2 : Ajouter le handler `get_profile`**

Juste avant `async fn get_balance`, ajouter :

```rust
async fn get_profile(
    req: HttpRequest,
    pool: web::Data<DbPool>,
) -> impl Responder {
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ").map(str::to_owned));

    let token = match token {
        Some(t) => t,
        None => return HttpResponse::Unauthorized()
            .json(serde_json::json!({"error": "missing token"})),
    };

    let jwt_secret = match env::var("JWT_SECRET") {
        Ok(s) => s,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let claims = match jsonwebtoken::decode::<ClaimsJWT>(
        &token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    ) {
        Ok(d) => d.claims,
        Err(_) => return HttpResponse::Unauthorized()
            .json(serde_json::json!({"error": "invalid token"})),
    };

    let user_id = claims.sub as i32;
    let result = web::block(move || {
        use crate::schema::users::dsl as u;
        let mut conn = pool.get().expect("DB pool");
        u::users
            .filter(u::id.eq(user_id))
            .select((u::username, u::avatar, u::money))
            .first::<(String, String, i64)>(&mut conn)
    })
    .await;

    match result {
        Ok(Ok((username, avatar, coins))) => {
            let avatar_url = if avatar.is_empty() { None } else { Some(avatar) };
            HttpResponse::Ok().json(ProfileResponse { username, avatar_url, coins })
        }
        _ => HttpResponse::InternalServerError()
            .json(serde_json::json!({"error": "db error"})),
    }
}
```

- [ ] **Step 3 : Enregistrer la route**

Dans `main()`, après la ligne `.route("/auth/balance", web::get().to(get_balance))`, ajouter :

```rust
.route("/auth/profile", web::get().to(get_profile))
```

- [ ] **Step 4 : Vérifier que le code compile**

```bash
cd oauth && cargo check 2>&1
```

Expected : aucune erreur. Si erreur de type sur le select tuple, vérifier que les types correspondent aux colonnes dans `schema.rs` (`username -> Varchar`, `avatar -> Text`, `money -> Int8` → `String`, `String`, `i64`).

- [ ] **Step 5 : Commit**

```bash
git add oauth/src/main.rs
git commit -m "feat(oauth): add GET /auth/profile endpoint"
```

---

## Task 2 — Fix redirect GitHub OAuth (ajouter `avatar_url`)

**Files:**
- Modify: `oauth/src/main.rs`

Le redirect GitHub actuel n'inclut pas `avatar_url`, donc le frontend ne peut pas le stocker. On l'ajoute.

- [ ] **Step 1 : Mettre à jour le `redirect_url` dans `github_callback`**

Trouver ce bloc (autour de la ligne 428) :

```rust
let redirect_url = format!(
    "{}?access_token={}&refresh_token={}&username={}",
    frontend_redirect,
    urlencoding::encode(&response_data.access_token),
    urlencoding::encode(&response_data.refresh_token),
    urlencoding::encode(&user.username),
);
```

Remplacer par :

```rust
let redirect_url = format!(
    "{}?access_token={}&refresh_token={}&username={}&avatar_url={}",
    frontend_redirect,
    urlencoding::encode(&response_data.access_token),
    urlencoding::encode(&response_data.refresh_token),
    urlencoding::encode(&user.username),
    urlencoding::encode(response_data.avatar_url.as_deref().unwrap_or("")),
);
```

- [ ] **Step 2 : Vérifier la compilation**

```bash
cd oauth && cargo check 2>&1
```

Expected : aucune erreur.

- [ ] **Step 3 : Commit**

```bash
git add oauth/src/main.rs
git commit -m "fix(oauth): include avatar_url in GitHub OAuth redirect params"
```

---

## Task 3 — Design tokens (`App.css`)

**Files:**
- Modify: `front-end/src/App.css`

- [ ] **Step 1 : Remplacer le bloc `:root`**

Trouver le bloc `:root { ... }` et le remplacer intégralement par :

```css
:root {
    color-scheme: dark;
    --bg-0: #0c0c0e;
    --bg-1: #111115;
    --panel: rgba(20, 20, 26, 0.88);
    --panel-strong: rgba(14, 14, 18, 0.96);
    --line: rgba(255, 255, 255, 0.07);
    --line-strong: rgba(196, 168, 130, 0.18);
    --text: #e8e8ea;
    --muted: rgba(232, 232, 234, 0.52);
    --accent: #c4a882;
    --shadow: 0 24px 56px rgba(0, 0, 0, 0.48);
    --radius-xl: 16px;
    --radius-lg: 12px;
    --radius-md: 8px;
}
```

- [ ] **Step 2 : Remplacer le background du body**

Trouver le bloc `body { ... }`. Remplacer la propriété `background:` (les 4 radial-gradient) par :

```css
background: linear-gradient(180deg, #111115 0%, #0c0c0e 100%);
```

- [ ] **Step 3 : Supprimer la texture grille**

Supprimer entièrement le bloc `body::before { ... }` (de `body::before {` jusqu'au `}` fermant).

- [ ] **Step 4 : Mettre à jour les boutons**

Trouver `.btn-primary, .btn-secondary { ... }` et changer `border-radius: 999px` en `border-radius: 8px`.

Trouver `.btn-primary { ... }` et remplacer son contenu par :

```css
.btn-primary {
    background: var(--accent);
    color: #1a1408;
    box-shadow: 0 6px 18px rgba(196, 168, 130, 0.18);
}
```

Trouver `.btn-secondary { ... }` et remplacer son contenu par :

```css
.btn-secondary {
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid var(--line);
    color: var(--text);
}
```

- [ ] **Step 5 : Vérifier visuellement**

```bash
cd front-end && npm run dev
```

Ouvrir http://localhost:5173. Le fond doit être noir neutre (plus de teinte verte), les boutons rectangulaires.

- [ ] **Step 6 : Commit**

```bash
git add front-end/src/App.css
git commit -m "style: apply new dark elegant design tokens"
```

---

## Task 4 — Header auth-aware

**Files:**
- Rewrite: `front-end/src/components/Header.jsx`
- Rewrite: `front-end/src/styles/Header.css`

- [ ] **Step 1 : Réécrire `Header.jsx`**

Remplacer intégralement le contenu de `front-end/src/components/Header.jsx` par :

```jsx
import { useNavigate, useLocation, Link, NavLink } from 'react-router-dom'
import '../styles/Header.css'

const NAV_PUBLIC  = [
  { to: '/',     label: 'Home'    },
  { to: '/rules', label: 'Règles' },
  { to: '/chat',  label: 'Chat'   },
]

const NAV_AUTH = [
  { to: '/',           label: 'Home'       },
  { to: '/friends',    label: 'Amis'       },
  { to: '/statistics', label: 'Stats'      },
  { to: '/history',    label: 'Historique' },
  { to: '/chat',       label: 'Chat'       },
  { to: '/rules',      label: 'Règles'     },
]

function Avatar({ url, username }) {
  if (url) {
    return <img src={url} alt={username} className="hdr-avatar-img" />
  }
  const initial = (username ?? 'J')[0].toUpperCase()
  return <span className="hdr-avatar-initials">{initial}</span>
}

export default function Header() {
  const navigate  = useNavigate()
  const location  = useLocation()
  const token     = localStorage.getItem('access_token')
  const isLoggedIn = !!token
  const username  = localStorage.getItem('username') ?? 'Joueur'
  const avatarUrl = localStorage.getItem('avatar_url') || null

  const navLinks = isLoggedIn ? NAV_AUTH : NAV_PUBLIC

  function handleLogout() {
    localStorage.clear()
    navigate('/login')
  }

  return (
    <header className="hdr">
      <Link to="/" className="hdr-brand">LUIGI JACK</Link>

      <nav className="hdr-nav">
        {navLinks.map(({ to, label }) => (
          <NavLink
            key={to}
            to={to}
            end={to === '/'}
            className={({ isActive }) => isActive ? 'hdr-link hdr-link--active' : 'hdr-link'}
          >
            {label}
          </NavLink>
        ))}
      </nav>

      <div className="hdr-right">
        {isLoggedIn ? (
          <>
            <Link to="/profile" className="hdr-user">
              <span className="hdr-avatar">
                <Avatar url={avatarUrl} username={username} />
              </span>
              <span className="hdr-username">{username}</span>
            </Link>
            <button className="hdr-logout" onClick={handleLogout} aria-label="Déconnexion">
              ⎋
            </button>
          </>
        ) : (
          <>
            <Link to="/login"    className="hdr-auth-link">Connexion</Link>
            <Link to="/register" className="btn-primary hdr-register">S'inscrire</Link>
          </>
        )}
      </div>
    </header>
  )
}
```

- [ ] **Step 2 : Réécrire `Header.css`**

Remplacer intégralement `front-end/src/styles/Header.css` par :

```css
.hdr {
  position: sticky;
  top: 0;
  z-index: 40;
  width: min(1160px, calc(100% - 32px));
  margin: 16px auto 0;
  padding: 10px 16px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  gap: 16px;
  background: var(--panel-strong);
  border: 1px solid var(--line);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.32);
  backdrop-filter: blur(16px);
}

.hdr-brand {
  text-decoration: none;
  color: var(--text);
  font-weight: 800;
  font-size: 0.95rem;
  letter-spacing: 0.08em;
  white-space: nowrap;
}

.hdr-nav {
  flex: 1;
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 2px;
  flex-wrap: wrap;
}

.hdr-link {
  text-decoration: none;
  padding: 8px 12px;
  border-radius: 6px;
  color: var(--muted);
  font-size: 0.9rem;
  font-weight: 500;
  letter-spacing: 0.02em;
  transition: background 0.15s, color 0.15s;
  position: relative;
}

.hdr-link:hover {
  color: var(--text);
  background: rgba(255, 255, 255, 0.04);
}

.hdr-link--active {
  color: var(--text);
}

.hdr-link--active::after {
  content: '';
  position: absolute;
  left: 12px;
  right: 12px;
  bottom: 4px;
  height: 2px;
  border-radius: 1px;
  background: var(--accent);
}

.hdr-right {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}

/* ── Logged-in ── */
.hdr-user {
  display: flex;
  align-items: center;
  gap: 8px;
  text-decoration: none;
  color: var(--text);
  padding: 6px 10px;
  border-radius: 8px;
  transition: background 0.15s;
}

.hdr-user:hover {
  background: rgba(255, 255, 255, 0.05);
}

.hdr-avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  overflow: hidden;
  flex-shrink: 0;
  border: 1px solid var(--line-strong);
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(196, 168, 130, 0.12);
}

.hdr-avatar-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.hdr-avatar-initials {
  font-size: 0.78rem;
  font-weight: 700;
  color: var(--accent);
  line-height: 1;
}

.hdr-username {
  font-size: 0.9rem;
  font-weight: 600;
}

.hdr-logout {
  background: none;
  border: 1px solid var(--line);
  border-radius: 6px;
  width: 34px;
  height: 34px;
  color: var(--muted);
  cursor: pointer;
  font-size: 1rem;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: border-color 0.15s, color 0.15s;
}

.hdr-logout:hover {
  border-color: rgba(255, 255, 255, 0.2);
  color: var(--text);
}

/* ── Logged-out ── */
.hdr-auth-link {
  text-decoration: none;
  padding: 8px 14px;
  border-radius: 8px;
  color: var(--muted);
  font-size: 0.9rem;
  font-weight: 500;
  transition: color 0.15s;
}

.hdr-auth-link:hover {
  color: var(--text);
}

.hdr-register {
  font-size: 0.88rem;
  min-height: 36px;
  padding: 0 14px;
}

@media (max-width: 820px) {
  .hdr {
    width: calc(100% - 18px);
    gap: 10px;
  }

  .hdr-username {
    display: none;
  }

  .hdr-nav {
    justify-content: flex-start;
  }
}
```

- [ ] **Step 3 : Supprimer l'import de `FaCog` (plus utilisé)**

Vérifier que le nouveau Header.jsx n'importe pas `react-icons`. Si l'ancienne version importait `FaCog`, l'import a été retiré dans la réécriture ci-dessus.

- [ ] **Step 4 : Vérifier visuellement**

```bash
cd front-end && npm run dev
```

- Non connecté (pas de token en localStorage) : le header affiche "Connexion" + "S'inscrire" à droite.
- Connecté (simuler : `localStorage.setItem('access_token', 'test')` + `localStorage.setItem('username', 'aloun')` dans la console DevTools) : affiche le username + bouton logout.
- Lien actif : underline doré sur la page courante.

- [ ] **Step 5 : Commit**

```bash
git add front-end/src/components/Header.jsx front-end/src/styles/Header.css
git commit -m "feat(header): auth-aware header with login/logout state"
```

---

## Task 5 — GameBoard — dealer CSS + visual refresh

**Files:**
- Modify: `front-end/src/components/GameBoard.jsx`
- Modify: `front-end/src/styles/GameBoard.css`

- [ ] **Step 1 : Remplacer le dealer dans `GameBoard.jsx`**

Supprimer la ligne :
```js
import dealerAvatar from "../assets/dealer.png"
```

Ajouter ce composant juste avant le composant `CardImage` :

```jsx
function DealerAvatar() {
  return (
    <div className="gb-dealer-avatar-css" aria-hidden="true">
      <svg viewBox="0 0 48 48" fill="none" xmlns="http://www.w3.org/2000/svg">
        {/* tête */}
        <circle cx="24" cy="16" r="9" fill="#2a2a34" stroke="rgba(196,168,130,0.3)" strokeWidth="1"/>
        {/* corps / costume */}
        <path d="M10 44 Q12 30 24 28 Q36 30 38 44" fill="#1e1e28" stroke="rgba(196,168,130,0.2)" strokeWidth="1"/>
        {/* nœud papillon */}
        <path d="M20 29.5 L24 31.5 L28 29.5 L24 27.5 Z" fill="#c4a882" opacity="0.85"/>
        {/* visage — yeux */}
        <circle cx="21" cy="15" r="1.2" fill="#c4a882" opacity="0.7"/>
        <circle cx="27" cy="15" r="1.2" fill="#c4a882" opacity="0.7"/>
      </svg>
    </div>
  )
}
```

Trouver dans la JSX du dealer zone :
```jsx
<div className="gb-dealer-avatar">
  <img src={dealerAvatar} alt="Croupier" />
</div>
```

Remplacer par :
```jsx
<div className="gb-dealer-avatar">
  <DealerAvatar />
</div>
```

- [ ] **Step 2 : Remplacer les emojis de la sidebar par des dots CSS**

Dans la JSX des table chips, trouver :
```jsx
{" — "}{t.phase === "Playing" ? "⚔️ en jeu" : t.open ? "🟢 ouverte" : "🔴 fermée"}
```

Remplacer par :
```jsx
{" — "}
<span
  className={`table-dot ${t.phase === "Playing" ? "table-dot--playing" : t.open ? "table-dot--open" : "table-dot--closed"}`}
/>
{t.phase === "Playing" ? "en jeu" : t.open ? "ouverte" : "fermée"}
```

- [ ] **Step 3 : Supprimer les emojis de l'outcome overlay**

Supprimer le composant `OUTCOME_CONFIG` existant et le remplacer par :

```js
const OUTCOME_CONFIG = {
  win:     { title: "Victoire",  cls: "outcome--win"  },
  lose:    { title: "Défaite",   cls: "outcome--lose" },
  neutral: { title: "Égalité",   cls: "outcome--tie"  },
}
```

Dans le JSX de `OutcomeScreen`, supprimer la ligne :
```jsx
<div className="outcome-emoji">{cfg.emoji}</div>
```

- [ ] **Step 4 : Mettre à jour `GameBoard.css`**

Appliquer les changements suivants dans `GameBoard.css` :

**4a — Sidebar** : changer `border-radius: 24px` → `border-radius: 12px` dans `.gb-sidebar`, et `border-radius: 16px` → `border-radius: 8px` dans `.table-chip`.

**4b — Arena** : remplacer le `background:` de `.gb-arena` par :
```css
background: #0e0e12;
border: 1px solid var(--line);
```

**4c — Dealer avatar CSS** : ajouter ces règles :
```css
.gb-dealer-avatar-css {
  width: 72px;
  height: 72px;
  border-radius: 50%;
  background: #1a1a22;
  border: 1px solid rgba(196, 168, 130, 0.2);
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
}

.gb-dealer-avatar-css svg {
  width: 58px;
  height: 58px;
}
```

**4d — Dots sidebar** : ajouter :
```css
.table-dot {
  display: inline-block;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  margin-right: 4px;
  vertical-align: middle;
}
.table-dot--playing { background: #e05c5c; }
.table-dot--open    { background: #6bcb8b; }
.table-dot--closed  { background: rgba(255,255,255,0.25); }
```

**4e — Boutons action** : dans `.gb-btn`, changer `border-radius: 999px` → `border-radius: 8px`.

Dans `.gb-btn--hit`, remplacer par :
```css
.gb-btn--hit {
  background: var(--accent);
  color: #1a1408;
}
```

Dans `.gb-btn--stay`, remplacer par :
```css
.gb-btn--stay {
  background: rgba(255, 255, 255, 0.07);
  color: var(--text);
  border: 1px solid var(--line);
}
```

**4f — Outcome overlay** : supprimer `.outcome-emoji { ... }`. Dans `.outcome-card`, changer `border-radius: 28px` → `border-radius: 12px`. Dans `.outcome-btn`, changer `border-radius: 999px` → `border-radius: 8px` et remplacer `background: linear-gradient(...)` par `background: var(--accent)`.

**4g — Board background** : dans `.gb-board`, remplacer le `background:` par :
```css
background: #111115;
border: 1px solid var(--line);
```

- [ ] **Step 5 : Vérifier visuellement**

```bash
cd front-end && npm run dev
```

Naviguer vers `/game`. L'arena doit être sombre et propre (plus de vert casino). Le dealer doit être une silhouette SVG sobre. Les boutons Hit/Stay doivent être rectangulaires.

- [ ] **Step 6 : Commit**

```bash
git add front-end/src/components/GameBoard.jsx front-end/src/styles/GameBoard.css
git commit -m "style(game): replace dealer photo with CSS avatar, refresh arena design"
```

---

## Task 6 — Supprimer les composants legacy

**Files:**
- Delete: `front-end/src/components/Dealer.jsx`
- Delete: `front-end/src/components/Table.jsx`
- Delete: `front-end/src/styles/Dealer.css`
- Delete: `front-end/src/styles/Table.css`

- [ ] **Step 1 : Vérifier qu'aucun fichier n'importe ces composants**

```bash
grep -r "Dealer\|Table" front-end/src --include="*.jsx" --include="*.js" -l
```

Expected : seuls `GameBoard.jsx` et `Game.jsx` peuvent apparaître (ils n'importent pas `Dealer.jsx` ni `Table.jsx`). Si un fichier les importe encore, le corriger avant de supprimer.

- [ ] **Step 2 : Supprimer les fichiers**

```bash
rm front-end/src/components/Dealer.jsx \
   front-end/src/components/Table.jsx \
   front-end/src/styles/Dealer.css \
   front-end/src/styles/Table.css
```

- [ ] **Step 3 : Vérifier que le build passe**

```bash
cd front-end && npm run build 2>&1 | tail -20
```

Expected : `built in Xs` sans erreur.

- [ ] **Step 4 : Commit**

```bash
git add -A front-end/src/components/Dealer.jsx \
           front-end/src/components/Table.jsx \
           front-end/src/styles/Dealer.css \
           front-end/src/styles/Table.css
git commit -m "chore: remove unused legacy Dealer and Table components"
```

---

## Task 7 — Home — overlay et titre

**Files:**
- Modify: `front-end/src/styles/Home.css`
- Modify: `front-end/src/pages/Home.jsx` (suppression du gradient or sur le titre)

- [ ] **Step 1 : Simplifier l'overlay dans `Home.css`**

Trouver `.home-overlay { ... }` et remplacer son `background:` par :

```css
background: linear-gradient(180deg, rgba(10, 10, 14, 0.28) 0%, rgba(10, 10, 14, 0.82) 100%);
```

- [ ] **Step 2 : Corriger le titre dans `Home.css`**

Trouver `.home-title span { ... }` et remplacer le bloc entier par :

```css
.home-title span {
  display: block;
  color: var(--accent);
}
```

(Supprimer les propriétés `-webkit-background-clip`, `-webkit-text-fill-color`, `background-clip`, `background:`.)

- [ ] **Step 3 : Mettre à jour les CTA dans `Home.jsx`**

Dans `Home.jsx`, les deux boutons utilisent déjà les classes `.btn-primary` et `.btn-secondary` — le redesign des tokens suffit. Vérifier que c'est bien le cas :

```bash
grep -n "btn-primary\|btn-secondary" front-end/src/pages/Home.jsx
```

Expected : les deux classes sont présentes. Rien à modifier si c'est le cas.

- [ ] **Step 4 : Vérifier visuellement**

```bash
cd front-end && npm run dev
```

Naviguer vers `/`. L'overlay doit être un dégradé sombre neutre (plus de teinte verte). "Jack" doit être en laiton mat, pas en gradient or criard.

- [ ] **Step 5 : Commit**

```bash
git add front-end/src/styles/Home.css
git commit -m "style(home): simplify overlay, remove gold gradient on title"
```

---

## Task 8 — Login + OAuthCallback

**Files:**
- Modify: `front-end/src/pages/Login.jsx`
- Modify: `front-end/src/App.jsx`
- Rewrite: `front-end/src/styles/Login.css`

- [ ] **Step 1 : Stocker `avatar_url` dans `Login.jsx`**

Dans le handler `handleSubmit`, après `localStorage.setItem("username", data.username)`, ajouter :

```js
localStorage.setItem("avatar_url", data.avatar_url ?? "")
```

- [ ] **Step 2 : Stocker `avatar_url` dans `OAuthCallback` (`App.jsx`)**

Dans `OAuthCallback`, trouver le bloc qui lit les params URL et ajouter :

```js
const avatarUrl = params.get("avatar_url")
if (avatarUrl) localStorage.setItem("avatar_url", avatarUrl)
```

Juste après les lignes existantes qui stockent `username`.

- [ ] **Step 3 : Mettre à jour la JSX de `Login.jsx`**

Supprimer la `<div className="login-glow" />`.

Remplacer `<span className="login-kicker">LUIGI JACK</span>` par rien (supprimer la ligne).

Changer la classe `login-heading` vers un simple `<div className="login-heading">` sans kicker.

- [ ] **Step 4 : Réécrire `Login.css`**

Remplacer intégralement le contenu de `front-end/src/styles/Login.css` par :

```css
.login-page {
  min-height: calc(100vh - 104px);
  display: grid;
  place-items: center;
  padding: 28px 16px 56px;
}

.login-card {
  width: min(440px, 100%);
  padding: 28px;
  border-radius: 12px;
  background: #141418;
  border: 1px solid rgba(255, 255, 255, 0.07);
  box-shadow: 0 20px 48px rgba(0, 0, 0, 0.4);
}

.login-heading {
  margin-bottom: 20px;
}

.login-heading h2 {
  margin: 0 0 6px;
  font-size: 1.6rem;
}

.login-subtitle {
  margin: 0;
  color: var(--muted);
  font-size: 0.95rem;
}

.login-form {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.login-form label {
  font-size: 0.75rem;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--muted);
}

.login-form input {
  padding: 12px 13px;
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.10);
  background: rgba(255, 255, 255, 0.04);
  color: var(--text);
  outline: none;
  transition: border-color 0.15s;
  font: inherit;
}

.login-form input::placeholder {
  color: rgba(232, 232, 234, 0.28);
}

.login-form input:focus {
  border-color: var(--accent);
}

.primary-button {
  margin-top: 4px;
  padding: 12px;
  border: none;
  border-radius: 8px;
  background: var(--accent);
  color: #1a1408;
  font-weight: 700;
  font: inherit;
  cursor: pointer;
  transition: opacity 0.15s;
}

.primary-button:hover:not(:disabled) {
  opacity: 0.88;
}

.primary-button:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.divider {
  display: flex;
  align-items: center;
  gap: 10px;
  color: rgba(232, 232, 234, 0.3);
  font-size: 0.78rem;
  letter-spacing: 0.14em;
  text-transform: uppercase;
}

.divider::before,
.divider::after {
  content: '';
  flex: 1;
  height: 1px;
  background: var(--line);
}

.github-button,
.register-link {
  padding: 12px;
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: #1a1a22;
  color: var(--text);
  font-weight: 500;
  font: inherit;
  cursor: pointer;
  transition: border-color 0.15s;
  text-align: center;
  text-decoration: none;
  display: block;
}

.github-button:hover,
.register-link:hover {
  border-color: rgba(255, 255, 255, 0.16);
}

.github-button:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.login-error {
  margin: 0;
  color: #f08080;
  font-size: 0.88rem;
}

.login-success {
  margin: 0;
  color: #80d8a8;
  font-size: 0.88rem;
}

@media (max-width: 480px) {
  .login-card {
    padding: 20px;
  }
}
```

- [ ] **Step 5 : Vérifier visuellement**

```bash
cd front-end && npm run dev
```

Naviguer vers `/login`. La card doit être sobre (#141418), sans halo, sans badge "LUIGI JACK". Inputs avec focus doré discret. Bouton GitHub sobre.

- [ ] **Step 6 : Commit**

```bash
git add front-end/src/pages/Login.jsx front-end/src/App.jsx front-end/src/styles/Login.css
git commit -m "style(login): sober card design, store avatar_url on login and OAuth"
```

---

## Task 9 — Register

**Files:**
- Modify: `front-end/src/pages/Register.jsx`

Register utilise déjà `Login.css` (réécrit à la tâche 8). Seul le JSX nécessite un ajustement.

- [ ] **Step 1 : Supprimer le `login-kicker` dans `Register.jsx`**

Trouver et supprimer :
```jsx
<span className="login-kicker">LUIGI JACK</span>
```

- [ ] **Step 2 : Supprimer le `<div className="login-glow" />`**

Trouver et supprimer :
```jsx
<div className="login-glow" />
```

- [ ] **Step 3 : Vérifier visuellement**

```bash
cd front-end && npm run dev
```

Naviguer vers `/register`. Doit être cohérent avec `/login`.

- [ ] **Step 4 : Commit**

```bash
git add front-end/src/pages/Register.jsx
git commit -m "style(register): remove legacy kicker and glow, align with login style"
```

---

## Task 10 — Profile — API réelle + redesign

**Files:**
- Rewrite: `front-end/src/pages/Profile.jsx`
- Rewrite: `front-end/src/styles/Profile.css`
- Delete: `front-end/src/assets/Aloun.jpg`, `Alexis.jpg`, `Adrien.jpg`, `JB.jpg`

- [ ] **Step 1 : Réécrire `Profile.jsx`**

Remplacer intégralement le contenu de `front-end/src/pages/Profile.jsx` par :

```jsx
import { useEffect, useState } from 'react'
import '../styles/Profile.css'

const AUTH_URL = import.meta.env.VITE_AUTH_URL ?? 'https://localhost:8443'

function AvatarDisplay({ url, username }) {
  if (url) {
    return <img src={url} alt={username} className="profile-avatar-img" />
  }
  const initial = (username ?? 'J')[0].toUpperCase()
  return <div className="profile-avatar-initials">{initial}</div>
}

export default function Profile() {
  const [data,    setData]    = useState(null)
  const [loading, setLoading] = useState(true)
  const [error,   setError]   = useState('')

  useEffect(() => {
    const token = localStorage.getItem('access_token')
    if (!token) { setError('Non authentifié'); setLoading(false); return }

    fetch(`${AUTH_URL}/auth/profile`, {
      headers: { Authorization: `Bearer ${token}` },
    })
      .then(r => r.ok ? r.json() : Promise.reject(r.status))
      .then(d => { setData(d); setLoading(false) })
      .catch(() => { setError('Impossible de charger le profil'); setLoading(false) })
  }, [])

  if (loading) {
    return (
      <div className="page-shell">
        <section className="page-card profile-page">
          <p className="profile-loading">Chargement…</p>
        </section>
      </div>
    )
  }

  if (error) {
    return (
      <div className="page-shell">
        <section className="page-card profile-page">
          <p className="profile-error">{error}</p>
        </section>
      </div>
    )
  }

  const isGitHub = !!data.avatar_url

  return (
    <div className="page-shell">
      <section className="page-card profile-page">
        <div className="profile-inner">

          <div className="profile-sidebar">
            <span className="section-chip">Profil</span>
            <div className="profile-avatar">
              <AvatarDisplay url={data.avatar_url} username={data.username} />
            </div>
            <h2 className="profile-name">{data.username}</h2>
            <p className="profile-account-type">
              {isGitHub ? 'Compte GitHub' : 'Compte local'}
            </p>
          </div>

          <div className="profile-stats">
            <div className="profile-stat-card">
              <span className="profile-stat-label">Coins</span>
              <strong className="profile-stat-value">{data.coins.toLocaleString()}</strong>
            </div>
            <div className="profile-stat-card">
              <span className="profile-stat-label">Pseudo</span>
              <strong className="profile-stat-value">{data.username}</strong>
            </div>
          </div>

        </div>
      </section>
    </div>
  )
}
```

- [ ] **Step 2 : Réécrire `Profile.css`**

Remplacer intégralement le contenu de `front-end/src/styles/Profile.css` par :

```css
.profile-page {
  padding: 28px;
}

.profile-inner {
  display: grid;
  grid-template-columns: 220px 1fr;
  gap: 24px;
  align-items: start;
}

/* ── Sidebar ── */
.profile-sidebar {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 12px;
  padding: 20px;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid var(--line);
}

.profile-avatar {
  width: 96px;
  height: 96px;
  border-radius: 12px;
  overflow: hidden;
  border: 1px solid var(--line-strong);
  flex-shrink: 0;
}

.profile-avatar-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.profile-avatar-initials {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(196, 168, 130, 0.1);
  font-size: 2rem;
  font-weight: 700;
  color: var(--accent);
}

.profile-name {
  margin: 0;
  font-size: 1.3rem;
}

.profile-account-type {
  margin: 0;
  font-size: 0.82rem;
  color: var(--muted);
}

/* ── Stats ── */
.profile-stats {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: 16px;
}

.profile-stat-card {
  padding: 20px;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid var(--line);
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.profile-stat-label {
  font-size: 0.75rem;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--muted);
}

.profile-stat-value {
  font-size: 1.4rem;
  color: var(--text);
}

.profile-loading,
.profile-error {
  padding: 40px;
  text-align: center;
  color: var(--muted);
}

.profile-error {
  color: #f08080;
}

@media (max-width: 700px) {
  .profile-inner {
    grid-template-columns: 1fr;
  }
}
```

- [ ] **Step 3 : Supprimer les assets hardcodés**

```bash
rm front-end/src/assets/Aloun.jpg \
   front-end/src/assets/Alexis.jpg \
   front-end/src/assets/Adrien.jpg \
   front-end/src/assets/JB.jpg
```

- [ ] **Step 4 : Vérifier que rien d'autre n'importe ces assets**

```bash
grep -r "Aloun\|Alexis\|Adrien\|JB" front-end/src --include="*.jsx" --include="*.js"
```

Expected : aucun résultat (les seuls imports étaient dans Profile.jsx et Friends.jsx).

Vérifier `Friends.jsx` — il importe ces 3 avatars. Les remplacer par des initiales ou une image placeholder :

Dans `front-end/src/pages/Friends.jsx`, supprimer les imports d'images et remplacer les `<img src={friend.avatar}>` par un div initiales :

```jsx
// Supprimer ces 3 lignes :
// import Aloun from "../assets/Aloun.jpg";
// import Alexis from "../assets/Alexis.jpg";
// import Adrien from "../assets/Adrien.jpg";

// Changer les données mock :
const friends = [
  { name: "Aloun",  online: true,  gains: "$12,450" },
  { name: "Alexis", online: true,  gains: "$21,900" },
  { name: "Adrien", online: false, gains: "$5,300"  },
]

// Remplacer <img src={friend.avatar} ... /> par :
// <div className="friend-avatar-initial">{friend.name[0]}</div>
```

Ajouter dans `front-end/src/styles/Friends.css` :
```css
.friend-avatar-initial {
  width: 48px;
  height: 48px;
  border-radius: 50%;
  background: rgba(196, 168, 130, 0.12);
  border: 1px solid var(--line-strong);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1.1rem;
  font-weight: 700;
  color: var(--accent);
  flex-shrink: 0;
}
```

- [ ] **Step 5 : Vérifier le build final**

```bash
cd front-end && npm run build 2>&1 | tail -20
```

Expected : build sans erreur ni warning d'import manquant.

- [ ] **Step 6 : Vérifier visuellement le Profile**

```bash
cd front-end && npm run dev
```

Naviguer vers `/profile` en étant connecté. Doit afficher username + coins depuis l'API, avatar initiale pour un compte local, avatar GitHub pour un compte OAuth.

- [ ] **Step 7 : Commit final**

```bash
git add front-end/src/pages/Profile.jsx \
        front-end/src/styles/Profile.css \
        front-end/src/pages/Friends.jsx \
        front-end/src/styles/Friends.css
git rm front-end/src/assets/Aloun.jpg \
       front-end/src/assets/Alexis.jpg \
       front-end/src/assets/Adrien.jpg \
       front-end/src/assets/JB.jpg
git commit -m "feat(profile): connect to GET /auth/profile, remove hardcoded mock data"
```

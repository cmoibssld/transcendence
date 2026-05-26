# Frontend Redesign — Moderne Sombre Élégant

**Date:** 2026-05-26  
**Scope:** Design system, Header (auth-aware), GameBoard, Home, Login, Register, Profile + nouvel endpoint `GET /auth/profile` (Rust)  
**Out of scope:** Settings, Statistics, History, Rules (traités plus tard)  
**Approach:** Option A — token swap + patches ciblés dans l'ordre

---

## 1. Design System (App.css)

### Palette

| Token | Ancienne valeur | Nouvelle valeur |
|---|---|---|
| `--bg-0` | `#060b0a` | `#0c0c0e` |
| `--bg-1` | `#0a1311` | `#111115` |
| `--panel` | `rgba(12,20,18,0.84)` | `rgba(20,20,26,0.88)` |
| `--panel-strong` | `rgba(8,13,12,0.94)` | `rgba(14,14,18,0.96)` |
| `--accent` | `#d6b15b` (or vif) | `#c4a882` (laiton mat) |
| `--accent-2` | `#5fd4c0` (teal) | supprimé |
| `--text` | `#f4efe5` | `#e8e8ea` |
| `--muted` | `rgba(244,239,229,0.68)` | `rgba(232,232,234,0.52)` |
| `--line` | `rgba(255,255,255,0.08)` | `rgba(255,255,255,0.07)` |
| `--radius-xl` | `28px` | `16px` |
| `--radius-lg` | `20px` | `12px` |
| `--radius-md` | `14px` | `8px` |

### Body background
Remplacer les 4 `radial-gradient` multicolores + le `linear-gradient` par :
```css
background: linear-gradient(180deg, #111115 0%, #0c0c0e 100%);
```

### Texture grille (`body::before`)
Supprimée entièrement.

### Boutons
- `border-radius: 999px` → `border-radius: 8px` partout (`.btn-primary`, `.btn-secondary`)
- `.btn-primary` : conserver le fond `--accent` mais sans le dégradé criard — fond plat `#c4a882`, texte `#1a1408`
- `.btn-secondary` : `background: rgba(255,255,255,0.05)`, `border: 1px solid var(--line)`

---

## 2. Header — Auth-aware

### Fichier : `src/components/Header.jsx` + `src/styles/Header.css`

### État non connecté
```
[LUIGI JACK]     Home · Rules · Chat        [Login] [Register]
```

### État connecté
```
[LUIGI JACK]     Home · Friends · Stats · History · Chat · Rules     [avatar] aloun [logout]
```

### Logique auth
- `isLoggedIn` = `!!localStorage.getItem('access_token')`
- `username` = `localStorage.getItem('username') ?? 'Joueur'`
- `avatarUrl` = `localStorage.getItem('avatar_url')` (stocké au login si l'API le retourne) ou fallback initiale

### Avatar
- Si `avatarUrl` disponible : `<img>` circulaire 36px
- Sinon : cercle CSS avec initiale du username, couleur `--accent`
- Suppression de `JB.jpg` hardcodé

### Logout
- Bouton icône `⎋` ou texte "Déconnexion"
- Action : `localStorage.clear()` + `navigate('/login')`

### Nav links
- Actif : underline `2px solid var(--accent)` sous le lien courant (via `useLocation`)
- Hover : fond `rgba(255,255,255,0.04)` rect arrondi `border-radius: 6px`
- Suppression du style pill `border-radius: 999px` sur les nav links

### Header shape
- `border-radius: 22px` → `border-radius: 12px`

---

## 3. GameBoard — Dealer & Arena

### Fichier : `src/components/GameBoard.jsx` + `src/styles/GameBoard.css`

### Dealer
- Supprimer `import dealerAvatar from "../assets/dealer.png"`
- Remplacer `<img src={dealerAvatar}>` par un composant `<DealerAvatar />` CSS pur :
  - Cercle `72px`, fond `#1a1a22`, border `1px solid rgba(196,168,130,0.2)`
  - SVG inline d'un buste de croupier stylisé (silhouette sobre, nœud papillon)

### Table / Arena
- Fond `.gb-arena` : `#0e0e12` sans image de fond
- Ovale central `.gb-center__ring` : `border: 2px solid rgba(196,168,130,0.12)`, fond transparent — dessiné en CSS pur
- Supprimer toute référence à `background.png` dans le contexte du jeu

### Sidebar
- Chips de table : supprimer emojis ⚔️🟢🔴, remplacer par dots CSS :
  - En jeu : `●` `#e05c5c`
  - Ouverte : `●` `#6bcb8b`
  - Fermée : `●` `rgba(255,255,255,0.25)`

### Outcome overlay
- Supprimer les emojis géants (🏆💀🤝)
- Titre en grand texte (`2rem`, `font-weight: 700`)
- Fond : `rgba(12,12,14,0.92)` backdrop-blur
- Bouton "Nouvelle manche" : style `.btn-primary` redesigné

### Boutons d'action (Hit / Stay / Quitter)
- `border-radius: 8px` (plus rectangulaire)
- Hit : fond `#c4a882`, texte sombre
- Stay : fond `rgba(255,255,255,0.07)`, border `var(--line)`
- Quitter : fond transparent, texte `rgba(232,232,234,0.45)`, border subtle

### Composants legacy à supprimer
- `src/components/Dealer.jsx` — non utilisé
- `src/components/Table.jsx` — non utilisé
- `src/styles/Dealer.css` — non utilisé
- `src/styles/Table.css` — non utilisé

---

## 4. Home

### Fichier : `src/pages/Home.jsx` + `src/styles/Home.css`

- `background.png` conservé en hero
- Overlay simplifié : `linear-gradient(180deg, rgba(10,10,14,0.3) 0%, rgba(10,10,14,0.78) 100%)` — supprimer le radial vert
- Titre "BlackJack" : blanc pur `#e8e8ea`, text-shadow discret `0 4px 24px rgba(0,0,0,0.6)` — supprimer le gradient or sur le texte
- CTAs : `.btn-primary` et `.btn-secondary` avec nouveaux styles rectangulaires
- Décoration `.home-cards-deco` (A♠ K♥ J♦) : conserver mais couleur `--accent` atténuée

---

## 5. Login / Register

### Fichiers : `src/pages/Login.jsx`, `src/pages/Register.jsx`, `src/styles/Login.css`

- Supprimer `.login-glow` (halo autour de la card)
- Card : `background: #141418`, border `1px solid rgba(255,255,255,0.07)`, `border-radius: 12px`
- Supprimer `.login-kicker` "LUIGI JACK" en majuscules — remplacer par un simple `<h2>Connexion</h2>`
- Inputs :
  - `background: rgba(255,255,255,0.04)`
  - `border: 1px solid rgba(255,255,255,0.10)`
  - focus : `border-color: var(--accent)`, `outline: none`
  - `border-radius: 8px`
- Bouton GitHub : fond `#1a1a22`, SVG GitHub inline à gauche, texte "Continuer avec GitHub"
- Bouton submit : `.btn-primary` redesigné

---

## 6. Nouvel endpoint — `GET /auth/profile` (Rust / oauth)

### Fichier : `oauth/src/main.rs`

Consolide username, avatar_url et coins en un seul appel authentifié.

### Réponse
```json
{ "username": "aloun", "avatar_url": "https://...", "coins": 1200 }
```
`avatar_url` est `null` pour les comptes locaux.

### Implémentation
1. Extraire le Bearer token de `Authorization` header
2. Décoder le JWT → récupérer `sub` (user_id) — réutiliser la logique déjà présente dans `introspect`
3. SELECT `username`, `avatar_url` depuis la table `users` WHERE `id = user_id`
4. Appeler la logique de balance existante (déjà dans `get_balance`) pour récupérer les coins
5. Retourner le JSON

### Route à ajouter
```
GET /auth/profile
```

---

## 7. Profile (frontend)

### Fichier : `src/pages/Profile.jsx` + `src/styles/Profile.css`

### Source de données
Un seul appel au mount :
```js
GET /auth/profile
Authorization: Bearer <access_token>
```
Retourne `{ username, avatar_url, coins }`.

### Avatar
- Si `avatar_url` non null (compte GitHub) : `<img>` circulaire
- Sinon (compte local) : cercle CSS avec initiale du username, couleur `--accent`
- Supprimer le sélecteur de 4 photos hardcodées et les assets `Aloun.jpg`, `Alexis.jpg`, `Adrien.jpg`, `JB.jpg`

### Données affichées
- Supprimer toutes les valeurs mockées (John Doe, Paris, $25,400, playerId "BJX-4207")
- Afficher : username, coins, type de compte (local ou GitHub selon `avatar_url`)
- Page en lecture seule — pas d'édition (aucun endpoint de mise à jour disponible)

### Login.jsx — fix requis
Ajouter le stockage de `avatar_url` à la réponse du login local :
```js
localStorage.setItem('avatar_url', data.avatar_url ?? '')
```
OAuthCallback : vérifier que `avatar_url` est bien passé dans les query params du redirect GitHub et le stocker également.

---

## Ordre d'implémentation

1. `oauth/src/main.rs` — `GET /auth/profile` endpoint
2. `App.css` — tokens, body, boutons
3. `Header.jsx` + `Header.css` — auth-aware
4. `GameBoard.jsx` + `GameBoard.css` — dealer, arena, boutons
5. Suppression des composants legacy (Dealer, Table)
6. `Home.css` — overlay, titre
7. `Login.jsx` + `Login.css` — fix avatar_url, card, inputs
8. `Register.jsx` — card, inputs
9. `Profile.jsx` + `Profile.css` — appel `/auth/profile`, suppression mocks

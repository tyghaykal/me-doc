# Google OAuth Login / Sign-up

**Goal:** Let users sign in or sign up with a Google account via OAuth 2.0
authorization-code flow (server-side, no Google Sign-In JS SDK — keeps the
frontend dependency-free and keeps the existing refresh-token session model).

## Flow

```
Browser ──GET /auth/google/login──▶ backend
    (redirect) ◀────────────────────────── 302 → accounts.google.com/o/oauth2/v2/auth
Google ──redirect──▶ backend /auth/google/callback?code&state
    (302 → https://<origin>/auth/google/callback?code&state)
Nuxt page /auth/google/callback ──reads refresh_token cookie + state──▶ applySession → /app
```

- `state` = random 32 bytes (URL-safe base64), stored in Redis `oauth:state:{hash}`
  with a short TTL, containing the PKCE code verifier (so no `nonce` table /
  column needed — Redis already exists).
- `/auth/google/callback` exchanges `code` for tokens, fetches `userinfo`,
  upserts the user by `google_sub` (else by verified email), marks email
  verified, creates a default workspace if none exists, issues the same
  session (`issue_session`) the OTP flow uses, then redirects the browser to
  the frontend's own callback page with the session cookie set.

## Database (new migration `0016_google_auth.sql`)

```sql
alter table users
    add column google_sub text,
    add column password_hash text,      -- now nullable (Google-only accounts)
    alter column email set not null;    -- email always present (userinfo provides it)
alter table users add constraint users_google_sub_unique unique (google_sub);
```

Already-nullable / already-set:
- `email_verified_at` — already nullable; Google accounts set it.
- `display_name`, `avatar_key` — already present (0008).

## Config (`.env` / `Config`)

| Var | Purpose |
|-----|---------|
| `GOOGLE_CLIENT_ID` | OAuth client ID |
| `GOOGLE_CLIENT_SECRET` | OAuth client secret |
| `GOOGLE_REDIRECT_URI` | e.g. `https://localhost/auth/google/callback` |

Backend `config.rs` reads them as optional (`Option<String>`); Google routes
404/redirect-to-login when unset (Google login disabled).

## Backend changes

- `src/auth/google.rs` (new): authorize URL builder, token exchange
  (reqwest — already a dep), userinfo fetch, state + PKCE helpers, upsert.
- `src/auth/mod.rs`: add `google` routes — `GET /auth/google/login` and
  `GET /auth/google/callback`. Both are **GET** (OAuth redirects can't POST),
  so they live in `auth::router()` (the non-sensitive router), NOT
  `sensitive_router()` (which is POST-only and rate-limited for
  credential-guessing; Google redirects are not brute-forceable).
- `password_hash` nullable → `register`/`login`/`change_password` guard:
  `password_hash is null` ⇒ "no password set; use Google" (`AuthError::NoPassword`).
- `UserResponse` gains optional `display_name`/`avatar_url` so the Google
  session carries the user's real name + avatar on first render.
- Nginx: `auth/google` already matches `location ~ ^/(health|auth|...)` — no
  nginx change needed.

## Frontend changes

- `login.vue` / `register.vue`: "Continue with Google" button (brand-matching
  white button, Google "G" logo inline SVG, same card layout) → full-page
  redirect to `${apiBase}/auth/google/login`.
- New page `app/pages/auth/google/callback.vue` (route `/auth/google/callback`):
  `auth.refresh()` (session cookie is set by then), then `navigateTo('/app')`.
  `guest` middleware excludes this page.
- `auth.ts` store: reuse existing `applySession`; add `avatar_url` to the
  `User` type so `useCollab`/`AppTopbar` presence shows the Google avatar.

## Tests

- `tests/google.rs`: with Google client credentials unset, `/auth/google/login`
  and `/auth/google/callback` must not panic and must redirect to login (or
  return a clear 4xx). Full token-exchange round-trip needs a Google sandbox
  account + real credentials — deferred (env-gated, skipped when unset).
- Existing `cargo test` must stay green.

## Verification

1. `cargo build` + `cargo test` in `backend/`.
2. `pnpm build` in `frontend/`.
3. Manual: set Google creds in `.env`, `docker compose up`, click
   "Continue with Google", complete Google's screen, land on `/app` with
   workspace created.

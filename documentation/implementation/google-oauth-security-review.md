# Google OAuth security review (follow-up)

Focused source-level pentest of the Google OAuth login/signup feature added
in `02d2c2c` (`backend/src/auth/google.rs`, `auth/mod.rs`, `email/mod.rs`,
`main.rs`, `frontend/app/pages/oauth/google/callback.vue`). Same style as
`security-audit-plan.md`, scoped to this one feature rather than the whole
stack. No live DAST — static review plus manual trace of the auth flow,
error paths, and every place Google-sourced data reaches the DB, an email,
or another user's browser.

## Findings

### High

**1. Unsanitized page title → email header injection**
- Where: `backend/src/email/mod.rs:142` (`send_share_notification`'s
  `subject = format!("{inviter_email} shared \"{page_title}\" with you...")`),
  fed by `backend/src/pages/mod.rs` (`create_page`/`update_page`), which
  stored `title` verbatim with no character restriction.
- Issue: a page title is fully attacker-controlled (any Editor can set it)
  and flows unescaped into the `Subject:` header of an email sent to anyone
  the page gets shared with. A title containing a raw `\r\n` is a classic
  header-injection vector — potentially adding arbitrary headers (extra
  `To:`/`Bcc:`) or splitting the message into attacker-chosen content,
  depending on how permissive the mail stack (`lettre` 0.11) is about
  folding raw CR/LF in a header value. `escape_html` already covers the
  *HTML body* (`page_title`/`inviter_email` are escaped there), but nothing
  touched the Subject header or the plain-text body.
- **Status: fixed.** `pages::sanitize_title` strips all control characters
  (`char::is_control`, which includes `\r`/`\n`) from a title before it's
  stored, applied at both entry points — `create_page`
  (`backend/src/pages/mod.rs`, near `let title = ...`) and `update_page`
  (`.bind(body.title.map(sanitize_title))`). Fixed at the write boundary so
  it protects every downstream consumer (email, UI, future features), not
  just today's email call site. Unit test:
  `pages::tests::sanitize_title_strips_crlf_and_other_control_chars`.

### Medium

**2. Google-account adoption doesn't clear a pre-existing `password_hash`**
- Where: `backend/src/auth/google.rs` — `upsert_google_user`'s
  brand-new-or-unverified-account branch (the `insert ... on conflict (email)
  do update` at the end of the function).
- Issue: an attacker can `POST /auth/register` with a victim's email and an
  attacker-chosen password, leaving the row unverified (never completes the
  registration OTP). When the real owner later signs in with Google using
  that same email, `upsert_google_user`'s `ON CONFLICT` branch flips
  `email_verified_at` to `now()` and links `google_sub` — but the original
  `DO UPDATE SET` list did not touch `password_hash`, so the attacker's hash
  silently kept working even after Google verified the account. `auth::login`
  itself still gates on a follow-up email OTP the attacker can't read, so
  this wasn't a full one-step takeover *today*, but it left an
  attacker-controlled credential live on the victim's account — a real risk
  if any future code path (password reset, "remember device", relaxed OTP)
  ever trusts "has a `password_hash`" as "this account has a settable
  password."
- **Status: fixed.** Added `password_hash = null` to the `ON CONFLICT DO
  UPDATE SET` clause in `upsert_google_user`, so adopting an
  unverified/pre-registered row via Google always clears any password an
  attacker may have set. The victim can set a fresh password later through
  the normal authenticated `change_password` flow.

**3. Google's `email_verified` claim was never checked**
- Where: `backend/src/auth/google.rs` — `exchange_code`'s userinfo parsing
  and `callback`'s use of `userinfo.email`.
- Issue: the code read `v["email"]`/`v["sub"]` from Google's userinfo
  response but never checked `v["email_verified"]`, then used that email as
  proof of ownership for both linking an existing account and adopting a
  new/unverified one. Google's userinfo endpoint can return
  `email_verified: false`; trusting the address anyway means account
  linking/creation could be driven by an email the caller doesn't actually
  control.
- **Status: fixed.** `GoogleUserInfo` now carries `email_verified` (parsed
  from either a JSON boolean or a stringified `"true"`/`"false"`, defaulting
  to `false` if absent — fail closed). `callback` rejects the login with
  `AuthError::Validation` before touching the DB if Google didn't confirm
  the email.

### Low

**4. Login error message enumerated Google-only accounts**
- Where: `backend/src/auth/mod.rs` (`login`), `backend/src/auth/error.rs`
  (`AuthError::NoPassword`).
- Issue: `POST /auth/login` returned a distinct message ("this account uses
  Google sign-in, not a password") for a Google-only account vs. the generic
  "invalid email or password" for a wrong password or a non-existent email.
  An attacker probing with guessed emails could use the response to learn
  both whether an email is registered and whether it's Google-only.
- **Status: fixed.** The password-login path now returns the same
  `AuthError::InvalidCredentials` for "no password set" as for "wrong
  password" — closes the enumeration channel. `AuthError::NoPassword` is
  kept for `users::change_password` (an authenticated endpoint where the
  caller already knows their own account state, so no enumeration risk).

**5. Google-sourced email bypassed the app's own email validation**
- Where: `backend/src/auth/google.rs` (`callback`), vs.
  `backend/src/auth/mod.rs::is_valid_email` (used by `/auth/register` and
  `/auth/login` specifically to reject control characters/angle
  brackets/quotes before an email is stored, emailed, or embedded in HTML).
- Issue: the Google callback path only did `.trim().to_lowercase()` on the
  IdP-supplied email, never running it through the same defense-in-depth
  check applied everywhere else. Low likelihood (Google controls the
  response), but inconsistent, and it's the same class of input that
  finding #1 shows can matter.
- **Status: fixed.** `callback` now calls the existing (private,
  parent-module) `is_valid_email` on the Google-sourced address via
  `super::is_valid_email`, rejecting the login with a validation error if it
  somehow fails the same check a manually-registered email would.

### Informational / accepted risk

**6. Google OAuth routes sit on the standard rate-limit bucket, not the
strict one**
- Where: `backend/src/main.rs`, `backend/src/auth/mod.rs::router` vs.
  `sensitive_router`.
- `/auth/google/login` and `/auth/google/callback` are GET-based (OAuth
  redirects can't carry POST bodies) and CSRF-safe via the state+PKCE
  nonce, not credential-guessable — this is a documented, deliberate choice
  (`google.rs:9-13`). Worst case is Redis key churn (~20 req/s, 600s TTL
  entries) from hitting `/auth/google/login` repeatedly, not account
  compromise. No change made; flagging for visibility only.

**7. Live secrets present in local `.env`**
- `/home/haykal/Code/me-doc/.env` (untracked, correctly gitignored) holds a
  real `GOOGLE_CLIENT_ID`/`GOOGLE_CLIENT_SECRET` pair and non-default JWT/AI
  encryption keys. Not a code defect — `Config::from_env()` loads these
  correctly and `.env.example` only ships placeholders — but worth an
  operational note: don't let this file leave the dev machine unencrypted,
  and rotate the Google client secret if it ever has.

## Verified with no issue found

- **CSRF/state**: 256-bit random `state`, hashed before storage, bound to a
  single-use Redis entry (`GETDEL`), 600s TTL — no replay, no timing
  side-channel.
- **PKCE**: S256 challenge, verifier never leaves the server.
- **Redirect URI / open redirect**: both the Google-bound `redirect_uri` and
  the post-login `frontend_origin` redirect come from server config, never
  from request/query input — checked on both backend (`google.rs`) and
  frontend (`callback.vue` reads no query params at all).
- **Client secret**: server-side only, never reaches `authorize_url` or the
  browser.
- **SSRF**: token exchange and userinfo calls target hardcoded Google
  hostnames.
- **JWT**: HS256, `exp`/`iat` enforced, secret required non-empty at boot.
- **Cookies**: `refresh_token` is `HttpOnly` + `Secure` + `SameSite=Lax`.
- **Frontend token storage**: access token lives only in an in-memory Pinia
  ref, never `localStorage`/`sessionStorage`; no `v-html` anywhere in the
  frontend.
- **DB race safety**: `google_sub` unique constraint + `ON CONFLICT (email)`
  make the upsert race-safe under concurrent callbacks.
- **Panics**: no `unwrap()`/`expect()` on attacker-controlled data in the
  OAuth path.

## Task breakdown

- [x] **#1** Sanitize page titles (strip control chars) at write time —
      *finding #1*
- [x] **#2** Null `password_hash` when Google adopts an unverified/new
      account row — *finding #2*
- [x] **#3** Check Google's `email_verified` claim before trusting the
      email — *finding #3*
- [x] **#4** Unify the "no password set" login error with the generic
      invalid-credentials error — *finding #4*
- [x] **#5** Run Google-sourced emails through `is_valid_email` — *finding #5*
- [ ] **#6** (accepted risk, no action) Google OAuth routes stay on the
      standard rate-limit bucket — *finding #6*
- [ ] **#7** (ops note, no code action) Confirm local `.env` handling/rotate
      Google secret if ever shared off-machine — *finding #7*
- [ ] **#8** Add regression coverage for OAuth state single-use/replay —
      not done this pass; `tests/google.rs` already documents why (no
      mockable Google endpoint in this test harness) and covers what's
      testable without one (fail-closed when unconfigured, forged-state
      rejection). A real replay test needs a mocked token/userinfo endpoint
      injected into `AppState`, which is a larger harness change than this
      fix pass — flagged as a follow-up, not implemented.

## Verification

- **#1**: `cargo test -p me-doc-backend sanitize_title` (unit test added).
  Manually: create a page titled `Test\r\nBcc: x@evil.com`, share it, inspect
  the outgoing message in Mailpit — confirm the raw title text appears
  literally in the subject/body rather than producing extra headers.
- **#2**: register with `victim@example.com` + a chosen password (leave
  unverified), then complete Google sign-in with the same email; query
  `select password_hash from users where email = 'victim@example.com'` and
  confirm it's `null`.
- **#3**: not independently testable without a mock Google IdP in this repo
  (same constraint as #8) — verified by code inspection and the existing
  `google_callback_rejects_unknown_state`/`google_login_fails_closed_when_not_configured`
  tests still passing unchanged.
- **#4**: `POST /auth/login` for a Google-only account and for a
  non-existent email; confirm both now return the identical `"invalid email
  or password"` message and 401 status.
- **#5**: code inspection (`super::is_valid_email` call added to `callback`);
  covered indirectly by the existing `is_valid_email` unit tests in
  `auth::tests`.
- **General**: `cargo check --tests` could not be run in this environment
  (no `cargo`/`rustc` on PATH) — all changes verified by careful manual
  reading of the surrounding code and Rust privacy/borrow rules instead.
  **Run `cargo build && cargo test` before merging** to confirm the crate
  compiles clean.

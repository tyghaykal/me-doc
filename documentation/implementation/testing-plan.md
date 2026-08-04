# MeDoc: Comprehensive Backend + Frontend E2E Test Suites

## Context

MeDoc (Rust/Axum backend, Nuxt/Vue frontend, Python/FastAPI converter service) currently has almost no automated testing: one backend file (`backend/tests/permissions.rs`) unit-tests permission resolution functions directly, and the frontend has zero test tooling at all. The request started as "test the import/export buttons" and was broadened to full end-to-end coverage of the whole app. The goal is comprehensive *scenario* coverage (not a measured code-coverage percentage) across both backend HTTP-level integration tests and frontend Playwright E2E tests, run against the existing local `docker-compose` dev stack — no new test infrastructure (no testcontainers, no CI wiring), and Playwright must drive its own bundled Chromium under WSL, never Windows/system Chrome.

## Decisions

| Question | Decision | Why |
|---|---|---|
| Router testability | Extract `pub fn build_app(state: AppState) -> Router` into `backend/src/lib.rs`; `main.rs` calls it, keeping CORS/rate-limit layers wrapped around the result in `main.rs` only | Smallest diff; tests don't want CORS or rate-limiting |
| Request-driving | `tower::ServiceExt::oneshot` for plain HTTP; a real bound port (`TcpListener` + `axum::serve`) only for the two websocket endpoints (`collab`, `comments::realtime`) | oneshot can't do a WS Upgrade handshake |
| Where backend tests run | `docker compose exec backend cargo test` | Container already reaches Postgres/Redis/MinIO/Mailpit/**converter** by service name — no new ports/mocks, especially needed for real import tests |
| OTP retrieval | Real round-trip through Mailpit's HTTP API, both backend and frontend | OTP is stored in Redis only as a SHA-256 hash (`backend/src/auth/otp.rs`) — no plaintext shortcut exists anywhere except the email body |
| Shared backend helpers | `backend/tests/common/mod.rs`, extending the `insert_user`/`insert_workspace`/... style already in `permissions.rs` | Standard Rust idiom; follows existing convention |
| Frontend runner | `@playwright/test`, Chromium project only, `baseURL: https://localhost` (via nginx) | Matches real cookie/CORS behavior vs. bypassing nginx on :3000 |
| Playwright `webServer` | None — assumes `docker compose up` is already running | Scripting compose lifecycle from Playwright duplicates existing dev workflow and risks racing migrations/minio bucket setup |
| Selectors | Role/text/attribute selectors already suffice almost everywhere; add `id`/`for` only to the 2-3 forms with multiple same-typed inputs and no distinguishing attribute | Real a11y improvement, not test-only scaffolding |
| Test data isolation | Backend: `#[sqlx::test]` gives each test its own ephemeral migrated DB. Frontend: every spec's fixture registers a brand-new unique account, so parallel specs never collide despite sharing one long-running DB/Redis/MinIO/Mailpit | No manual cleanup needed anywhere; growth in Redis/Mailpit/MinIO is cosmetic only |

## Backend: harness changes

1. **`backend/src/lib.rs`** — add `pub fn build_app(state: AppState) -> axum::Router` containing exactly the router-assembly currently inlined in `main.rs:90-107` (routes + `CookieManagerLayer` + `TraceLayer`, *not* CORS/`GovernorLayer`).
2. **`backend/src/main.rs`** — replace the inline `Router::new()...` block with `let app = me_doc_backend::build_app(state);`, then still apply `.layer(cors)` etc. around it as today.
3. **`backend/Cargo.toml`** — add `[dev-dependencies]`: `tower = { version = "0.4", features = ["util"] }` (oneshot), `http-body-util = "0.1"` (reading oneshot response bodies), `tokio-tungstenite = "0.24"` (WS client, collab/comments-realtime tests only). Reuse the existing `reqwest` main-dep for Mailpit API calls and any real-port HTTP setup.
4. **`backend/tests/common/mod.rs`** (new) — move `insert_user`/`insert_workspace`/`insert_page`/`grant_page`/`add_member`/`grant_page_link` out of `permissions.rs` here unchanged (update `permissions.rs` to `mod common; use common::*;`). Add:
   - `test_state(pool: PgPool) -> AppState` — builds `AppState` from container env (`Config::from_env()`, live Redis/S3/Email clients) with the given `#[sqlx::test]` pool substituted in.
   - `register_and_login(app: &Router, email: &str, password: &str) -> AuthedClient` — full register → Mailpit OTP → verify → login → Mailpit OTP → verify flow; the most-reused helper.
   - `mailpit_latest_code(to: &str) -> String` — polls `http://mailpit:8025/api/v1/search?query=to:{to}` (container-internal hostname, distinct from the host-published `localhost:8125` the frontend suite uses), extracts the 6-digit code from the newest matching message.
   - `spawn_real_server(state: AppState) -> (SocketAddr, JoinHandle<()>)` — binds an ephemeral port for the two WS test files.
5. **Fixtures**: small checked-in files under `backend/tests/fixtures/` (a tiny real `.docx` and `.md`) for import tests.

## Backend: test files (`backend/tests/*.rs`, one per feature area)

Each follows the existing `#[sqlx::test]`-per-test pattern (fresh DB per test, no manual cleanup):

- `auth.rs` — register→OTP→verify→login→OTP→verify happy path; wrong/expired OTP; cooldown; refresh rotation; logout; duplicate-email rejected.
- `users.rs` — `GET/PATCH /auth/me`; password change (wrong current password rejected); avatar presign/download workspace-access gating.
- `workspaces.rs` — create; list scoped to membership; add/remove member; non-owner blocked from managing members.
- `pages.rs` — CRUD; trash/restore; duplicate; content PUT/GET round-trip; favorite/unfavorite; search scoped per-workspace (cross-workspace isolation); shared/favorite listings.
- `sharing.rs` — share by email (existing + pending pre-signup); share-link create + access; permission edit/delete blocked for non-owner; Viewer can't escalate.
- `comments.rs` — create/list/resolve/delete; resolve blocked for non-Editor; access denied on a page the user can't reach.
- `collab.rs` (real port) — valid token/link connects; invalid/missing rejected; two clients converge on the same sync-step broadcast.
- `comments_realtime.rs` (real port) — WS connects with valid auth; REST-posted comment propagates to the stream.
- `convert.rs` (import) — real `.docx`/`.md` fixture succeeds via the real converter service; oversized (>20MB) rejected; unsupported extension rejected.
- `export.rs` — md/docx/pdf succeed for Editor with correct `Content-Disposition`; **Viewer rejected with 403** (the one explicit permission edge case already called out).

## Frontend: setup

1. **`frontend/package.json`** — add `"@playwright/test": "^1.48.0"` to devDependencies. Do not run `npx playwright install` — the bundled Chromium is already cached at `~/.cache/ms-playwright/chromium-1223`; config just uses the default `devices['Desktop Chrome']`.
2. **`frontend/playwright.config.ts`** (new) — `testDir: './tests/e2e'`, `fullyParallel: true`, `use: { baseURL: 'https://localhost', ignoreHTTPSErrors: true }`, single `chromium` project, no `webServer` block.
3. **`frontend/tests/e2e/`** layout:
   - `fixtures.ts` — extends `test` with an `authedPage` fixture: registers a unique account via UI, polls Mailpit (`http://localhost:8125`, host-published port) for the OTP, verifies, lands in `/app`. Every spec besides `auth.spec.ts` imports `test` from here instead of `@playwright/test` directly.
   - `helpers/mailpit.ts` — `pollForOtp(email)`.
   - `helpers/ids.ts` — `uniqueEmail()`/`uniqueName()`.
   - `fixtures/` — small checked-in files for import tests.
   - Spec files: `auth.spec.ts`, `pages-editor.spec.ts`, `sharing.spec.ts`, `comments.spec.ts`, `collab.spec.ts` (two browser contexts), `import.spec.ts`, `export.spec.ts`, `settings.spec.ts`.
4. **Selector fixes**: grep modals for multiple same-typed inputs lacking `id`/`for` (expected candidates: `UserSettingsModal.vue` password fields, possibly `ShareDialog.vue`/`CreateWorkspaceModal.vue`) and add real `id`/`for` pairs — a genuine a11y fix, done only where ambiguity actually exists.

## Running the suites

```bash
docker compose up -d
docker compose run --rm minio-createbuckets   # once, on a fresh minio-data volume

docker compose exec backend cargo test                # backend, all
docker compose exec backend cargo test --test export  # backend, one file

cd frontend && npm install
npx playwright test                                     # frontend, all
npx playwright test tests/e2e/auth.spec.ts               # frontend, one spec
```
Add a short "Testing" section to `README.md` documenting these commands (no CI file).

## Verification

- `docker compose exec backend cargo test` passes for every new test file, run against the live dev stack.
- `npx playwright test` passes headless against the running `docker compose` stack, using Playwright's own WSL-local Chromium.
- Spot-check the collab and comments-realtime WS tests manually once if anything about the real-port harness seems flaky.
- Confirm `permissions.rs` still passes unchanged after its helpers move into `tests/common/mod.rs`.

## Critical files

- `backend/src/lib.rs`, `backend/src/main.rs`, `backend/Cargo.toml`
- `backend/tests/common/mod.rs` (new), `backend/tests/permissions.rs` (updated), `backend/tests/{auth,users,workspaces,pages,sharing,comments,collab,comments_realtime,convert,export}.rs` (new)
- `frontend/package.json`, `frontend/playwright.config.ts` (new)
- `frontend/tests/e2e/{fixtures.ts,helpers/*,*.spec.ts}` (new)

## Execution plan (task breakdown for parallel agents)

**Wave 1 — foundational (parallel, must finish before Wave 2):**
1. `backend-harness` — lib.rs `build_app`, main.rs update, Cargo.toml dev-deps, `tests/common/mod.rs` (helpers moved + new), `tests/fixtures/` sample docx/md, update `permissions.rs` to use `common`. Verify `cargo test --test permissions` still passes.
2. `frontend-setup` — package.json devDependency, `playwright.config.ts`, `tests/e2e/fixtures.ts`, `tests/e2e/helpers/{mailpit,ids}.ts`, `tests/e2e/fixtures/` sample files, selector audit + `id`/`for` fixes in modals.

**Wave 2 — parallel implementation, grouped by shared context:**
3. `backend-auth-users-workspaces` — `tests/auth.rs`, `tests/users.rs`, `tests/workspaces.rs`
4. `backend-pages-sharing` — `tests/pages.rs`, `tests/sharing.rs`
5. `backend-comments-collab` — `tests/comments.rs`, `tests/collab.rs`, `tests/comments_realtime.rs`
6. `backend-import-export` — `tests/convert.rs`, `tests/export.rs`
7. `frontend-auth-editor` — `auth.spec.ts`, `pages-editor.spec.ts`
8. `frontend-sharing-comments` — `sharing.spec.ts`, `comments.spec.ts`
9. `frontend-collab-settings` — `collab.spec.ts`, `settings.spec.ts`
10. `frontend-import-export` — `import.spec.ts`, `export.spec.ts`

**Wave 3 — verification:** run both full suites, fix any failures, add the README "Testing" section.

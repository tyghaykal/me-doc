# me-doc — Full End-to-End Implementation Plan

## Context

`architecture.md` describes the target system; this doc is the concrete, phase-by-phase execution plan to get there from where the codebase actually stands today, verified by direct exploration (not assumed from the architecture doc alone). It exists so future work (this session or a new one) can pick up any phase without re-deriving the current state or re-litigating design decisions already made here.

## Current state (verified)

**Backend** (`backend/src/`) — Phase 1 + most of Phase 2 done:
- Implemented: `/health` (Postgres+Redis check), full `auth/` module — register → email OTP → verify → JWT + httpOnly refresh cookie; login → OTP → verify; refresh-token rotation (hashed in Postgres, revocable); logout. Argon2 password hashing, `jsonwebtoken` access tokens, Redis-backed OTP (SHA256 hash, 60s cooldown, 5 max attempts). Default workspace auto-created on register (`workspaces::create_default_workspace`).
- `workspaces/mod.rs` (35 lines) has only `create_default_workspace` — no CRUD, no router.
- `collab/`, `pages/`, `export/`, `sharing/`, `storage/` are empty stub directories, zero files, not mounted in `main.rs`.
- One migration (`migrations/0001_init.sql`): `users`, `workspaces`, `workspace_members` (role check `owner/admin/member/guest`), `refresh_tokens`. No `pages`, `page_content`, `page_versions`, `permissions`, `attachments` yet.
- `Cargo.toml` has axum 0.7 (ws feature already on), sqlx 0.7, redis 0.25, jsonwebtoken 9, argon2, lettre 0.11. **Not yet added**: `yrs`, `yrs-axum`, `aws-sdk-s3`, `docx-rs`, `chromiumoxide`.
- No auth-required extractor exists yet — every current route is either public or issues auth itself. Phase 3 needs the first "requires a valid session" extractor.

**Frontend** (`frontend/app/`) — Phase 1 scaffold + Tailwind v4/SSR-CSR split done, nothing else:
- `pages/index.vue` (SSR landing, Tailwind-styled, calls `/health`), `pages/app/index.vue` (CSR placeholder shell).
- `composables/useApiBase.ts` is the only composable — no auth store, no fetch wrapper, no session handling.
- `components/` and `stores/` are empty directories. No Pinia, no Tiptap, no Yjs client deps in `package.json`.
- None of the architecture-doc pages exist: `/register`, `/verify-otp`, `/login`, `/login/otp`, workspace dashboard, page tree, editor, share dialog, export menu.
- `.nvmrc` pins Node 22 (required — see cross-cutting fix below).

**Infra** — fully wired for Phase 1, validated (`docker compose config` passes clean):
- `docker-compose.yml`: postgres, redis, minio + `minio-createbuckets` (one-shot init, matches architecture.md's requirement), mailpit, backend, frontend — all with healthchecks and `depends_on: service_healthy` ordering.
- `docker-compose.override.yml`: dev hot-reload (cargo-watch for backend, `pnpm dev --host` for frontend).
- Both backend and frontend have `Dockerfile` (prod, multi-stage) and `Dockerfile.dev`. Backend's prod image already installs chromium + fonts-liberation, anticipating Phase 7 export before `chromiumoxide` is even in `Cargo.toml`.

## Cross-cutting fix needed before/with Phase 3

`frontend/Dockerfile` and `frontend/Dockerfile.dev` currently pin `node:20-slim`. This project's Nuxt 4 dependency chain (`oxc-parser`/`oxc-walker`) needs Node 22+ for synchronous `require()` of ESM — confirmed by direct reproduction during the Tailwind work (both `nuxt dev` and `nuxt build` crash on Node 20, even with zero custom config). Bump both Dockerfiles' base image to `node:22-slim` — otherwise Docker-based dev/build will hit the exact crash already diagnosed and fixed on the host.

## Phase 2 (completion): Auth frontend

Backend auth is done; nothing there needs to change. Build the frontend to actually use it.

- **Dependency**: add `pinia` + `@pinia/nuxt` (module) — the only new frontend dependency needed for this phase.
- **`app/stores/auth.ts`** (new) — Pinia store holding the in-memory access token + current user; actions `register`, `verifyOtp`, `login`, `loginVerify`, `logout`, `refresh`. Mirrors the backend's `AuthResponse` shape (`access_token`, `user`, `workspace`) already returned by `auth/mod.rs`.
- **`app/composables/useApi.ts`** (new) — wraps Nuxt's `$fetch`/`useFetch` with `credentials: 'include'` (refresh cookie is httpOnly, scoped to `/auth`), attaches `Authorization: Bearer <access_token>` from the auth store, and on a 401 does one silent `POST /auth/refresh` + retry before giving up. This is the reusable fetch layer every later phase's API calls build on — get it right once here.
- **Pages** (all under `app/pages/`, matching architecture.md's route list): `register.vue`, `verify-otp.vue`, `login.vue`, `login/otp.vue`. Straightforward forms posting to the auth store actions above.
- **Route guard**: a Nuxt route middleware (`app/middleware/auth.ts`) applied to `/app/**` (already CSR-only via existing `routeRules`) that redirects to `/login` if there's no valid session; attempts one `refresh()` first (e.g. on hard page load where the Pinia store is empty but the refresh cookie may still be valid).

**Files touched**: `frontend/package.json` (pinia), `frontend/nuxt.config.ts` (add `@pinia/nuxt` to `modules`), `app/stores/auth.ts`, `app/composables/useApi.ts`, `app/pages/{register,login,verify-otp,login/otp}.vue`, `app/middleware/auth.ts`.

## Phase 3: Workspaces & pages

**Migration** (`migrations/0002_pages.sql`): 
```sql
create table pages (
    id uuid primary key default gen_random_uuid(),
    workspace_id uuid not null references workspaces(id) on delete cascade,
    parent_page_id uuid references pages(id) on delete cascade,
    title text not null default 'Untitled',
    slug text not null,
    order_index integer not null default 0,
    archived_at timestamptz,
    created_by uuid not null references users(id),
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);
create index idx_pages_workspace_id on pages(workspace_id);
create index idx_pages_parent_page_id on pages(parent_page_id);
```

**Backend**:
- First auth-required extractor: `AuthenticatedUser` in `src/auth/extractor.rs` (new), implementing `FromRequestParts`, reading the `Authorization: Bearer` header and calling the existing `jwt::verify_access_token`. Every route from here on takes this as an extractor argument instead of hand-parsing headers.
- `src/workspaces/mod.rs` — extend with a `router()` (currently has none) exposing list/get for the current user's workspaces, mounted at `/workspaces` in `main.rs`.
- `src/pages/mod.rs` (currently empty) — CRUD + tree endpoints: `POST /workspaces/:id/pages` (create, optional `parent_page_id`), `GET /workspaces/:id/pages` (flat list, frontend builds the tree client-side — simplest given page counts per workspace are small), `PATCH /pages/:id` (title/order_index/parent_page_id — reparent+reorder in one endpoint), `DELETE /pages/:id` (soft delete via `archived_at`, not hard delete — needed later for Phase 8 trash/restore anyway, cheaper to do it right the first time than retrofit).
- Workspace-membership check: for now a simple per-handler query against `workspace_members` (role doesn't matter yet, any membership grants access) — this gets superseded by the real permission-resolution engine in Phase 6, so don't over-build it here.

**Frontend**:
- `app/stores/pages.ts` (Pinia) — holds the flat page list for the active workspace, computed tree structure, active-page id.
- `app/components/PageTree.vue` — recursive sidebar component (renders itself for children). Start with native HTML5 drag-and-drop (`draggable`, `dragstart`/`dragover`/`drop` events) for reorder/reparent — no new dependency needed for this; only reach for a drag-and-drop library later if native events prove too fiddly for nested-list semantics.
- `app/pages/app/index.vue` (replaces the current placeholder) — workspace dashboard: `PageTree` sidebar + main content area.

## Phase 4: Editor (static first)

**Migration** (`migrations/0003_page_content.sql`):
```sql
create table page_content (
    page_id uuid primary key references pages(id) on delete cascade,
    yjs_state bytea not null default '\x',
    plain_text text,
    updated_at timestamptz not null default now()
);
```
(`plain_text` as plain `text` for now; convert to `tsvector` + GIN index in Phase 8 once search is actually being built — no point maintaining a search index nobody queries yet.)

**Backend**:
- `src/storage/mod.rs` — S3 client wrapper around MinIO using `aws-sdk-s3` (add to `Cargo.toml`); one function, presigned PUT URL generation for a given `workspace_id`/`attachment_id` key.
- `migrations/0004_attachments.sql` — the `attachments` table from architecture.md.
- `src/pages/mod.rs` additions: `GET /pages/:id/content` (returns raw `yjs_state` bytes or empty), `POST /attachments/presign` (returns a presigned PUT URL + the final public/proxy GET URL).
- Manual save only in this phase — no WebSocket yet. The editor's Yjs doc state gets PUT to `/pages/:id/content` on a debounce; Phase 5 replaces "manual save" with realtime sync but keeps the same underlying `page_content.yjs_state` column as the persistence target, so this isn't wasted work.

**Frontend**:
- Dependencies: `@tiptap/vue-3`, `@tiptap/starter-kit`, `@tiptap/extension-image`, `yjs` (needed even before Phase 5, since Tiptap's collaborative-ready document model is a Yjs doc from the start — avoids a document-model rewrite when Phase 5 adds the network layer).
- `app/components/Editor.vue` — Tiptap wrapper bound to a `Y.Doc`, image paste/drop → presign → S3 PUT → insert image node, debounced save of `Y.encodeStateAsUpdate(doc)` to the content endpoint.

## Phase 5: Realtime collaboration

Confirmed available crates (verified live, not from training-data memory): `yrs` (core CRDT, v0.27.x) and `yrs-axum` (v0.8.x, axum-specific WebSocket sync-protocol glue, successor to `yrs-warp`) — this is exactly the "reuse before building" case: **do not hand-roll the Yjs sync/awareness wire protocol**, `yrs-axum` already implements it and is wire-compatible with JS Yjs WebSocket providers.

**Backend**:
- Add `yrs = "0.27"`, `yrs-axum = "0.8"` to `Cargo.toml`.
- `AppState` gains a `docs: Arc<DashMap<Uuid, Arc<yrs_axum::BroadcastGroup>>>` registry (add the `dashmap` crate) — one `BroadcastGroup` per actively-edited page, created lazily on first connection.
- `src/collab/mod.rs`: `GET /ws/pages/:id` WebSocket route (axum's `ws::WebSocketUpgrade`, already enabled as a Cargo feature). On connect: authenticate (JWT as a query param, since browser WebSocket clients can't set custom headers — validate with the existing `jwt::verify_access_token`), look up or create the page's `BroadcastGroup` (loading initial state from `page_content.yjs_state` if this is the first connection), hand the socket to `yrs_axum`'s connection handler which speaks the sync/awareness protocol and broadcasts to peers automatically.
- Persistence: a debounced task per `BroadcastGroup` (e.g. flush 5s after the last update, or immediately on last-client-disconnect) writes `Y.encode_state_as_update` back to `page_content.yjs_state`. Structure this as a single `persist_snapshot(page_id)` function from day one — Phase 8's `page_versions` history just calls the same function into a new table row instead of an UPDATE, so this isn't a rearchitect later.
- Evict a page's `BroadcastGroup` from the `DashMap` once its last client disconnects (after the final persist) — keeps memory bounded to actively-edited pages, not every page ever opened.

**Frontend**:
- Add `y-websocket`'s `WebsocketProvider` (or a thin custom provider if the exact JS package proves awkward with Nuxt SSR — since `/app/**` is CSR-only this is moot, the provider only ever runs client-side) pointed at `ws://.../ws/pages/:id?token=<access_token>`, plus `@tiptap/extension-collaboration` + `@tiptap/extension-collaboration-cursor` wired to the same `Y.Doc` the editor already uses from Phase 4.
- Presence/cursors come for free from the awareness protocol once the provider is connected — no custom presence code needed.

## Phase 6: Sharing & permissions

**Migration** (`migrations/0005_permissions.sql`):
```sql
create table permissions (
    id uuid primary key default gen_random_uuid(),
    subject_type text not null check (subject_type in ('workspace','page')),
    subject_id uuid not null,
    principal_type text not null check (principal_type in ('user','link')),
    principal_id uuid,              -- user id, null for link grants
    link_token text unique,         -- set only when principal_type = 'link'
    role text not null check (role in ('viewer','editor')),
    expires_at timestamptz,
    created_at timestamptz not null default now()
);
create index idx_permissions_subject on permissions(subject_type, subject_id);
create index idx_permissions_link_token on permissions(link_token) where link_token is not null;
```

**Resolution algorithm** — a single recursive CTE, not app-level loop queries (one round trip, and Postgres already has to walk the same `parent_page_id` chain that `PageTree` uses client-side):
```sql
with recursive page_chain as (
    select id, parent_page_id, workspace_id, 0 as depth
    from pages where id = $1
    union all
    select p.id, p.parent_page_id, p.workspace_id, pc.depth + 1
    from pages p join page_chain pc on p.id = pc.parent_page_id
)
select perm.role
from page_chain pc
join permissions perm
  on (perm.subject_type = 'page' and perm.subject_id = pc.id)
where perm.principal_id = $2 or perm.link_token = $3
order by pc.depth asc
limit 1
-- if no row: fall back to a second query against workspace_members.role for the page's workspace_id
```
First match walking from the page upward wins (page-level override beats workspace-level, closer ancestor beats farther); if nothing matches anywhere in the chain, fall back to `workspace_members`.

**Backend**:
- `src/sharing/mod.rs`: a `PagePermission` extractor (Rust `FromRequestParts`) usable on every page/content/collab route going forward — takes either the `AuthenticatedUser` extractor's `user_id` or a `?link=<token>` query param, runs the CTE above, and rejects with 403 if no role resolves. This replaces the "any workspace member can access" placeholder check from Phase 3.
- Routes: `POST /pages/:id/share` (grant a user a role by email), `POST /pages/:id/share/link` (create a public link-token grant), `DELETE /permissions/:id` (revoke).
- Retrofit: every Phase 3/4/5 route (`pages/*`, `/ws/pages/:id`) swaps its placeholder membership check for the `PagePermission` extractor.

**Frontend**: `app/components/ShareDialog.vue` (invite by email + role picker, generate/copy public link), wired into the page toolbar.

## Phase 7: Export

**Backend** (`src/export/mod.rs`, currently empty):
- Markdown: custom serializer walking the Tiptap/ProseMirror JSON document (fetched by decoding the Yjs doc's XML fragment) — no external dependency, matches architecture.md's stated approach.
- DOCX: add `docx-rs` to `Cargo.toml`, map the same document JSON to `docx-rs` builder calls.
- PDF: add `chromiumoxide` to `Cargo.toml` (the Docker image already has chromium installed for exactly this). Server renders the page's content to a standalone HTML string (reuse the frontend's read-only rendering, or a minimal server-side template — decide based on how much editor-specific CSS the PDF needs to look right), then `chromiumoxide` prints it to PDF.
- Routes: `GET /pages/:id/export?format=md|docx|pdf`, gated by the same `PagePermission` extractor (viewer role is enough to export).

**Frontend**: export menu (dropdown in the page toolbar) triggering a download via the export endpoint.

## Phase 8: Polish

- **Search**: convert `page_content.plain_text` to `tsvector` (generated column) + GIN index; `GET /workspaces/:id/search?q=`; simple search box in the dashboard.
- **Trash/restore**: already have `archived_at` on `pages` from Phase 3 — just needs a "trash" view (`archived_at is not null`) and a restore endpoint (`PATCH /pages/:id/restore`).
- **Version history/rollback**: `page_versions` table (id, page_id, snapshot bytea, version_no, created_at) — the Phase 5 `persist_snapshot(page_id)` function gets a second call site that also inserts a row here (e.g. every Nth snapshot or once per session) instead of only updating `page_content`.
- **Rate limiting**: add `tower-governor` (or similar tower-compatible crate) as middleware on `main.rs`'s router — not present in `Cargo.toml` yet.
- **Tests**: no test infrastructure exists at all yet. Start with `sqlx::test` (built into the `sqlx` feature set already in `Cargo.toml`, no new dependency) for a handful of the highest-value paths: the permission-resolution CTE (Phase 6, the trickiest logic in the whole system) and the auth OTP flow.

## Suggested order of work

Phase 2 (auth frontend) first — it's the smallest increment and unlocks testing every later phase through a real logged-in session instead of curl-ing tokens by hand. After that, phases 3→8 in the order above track a hard dependency chain (pages before content, content before collab, collab's persistence hook before version history) — no phase can jump ahead of the one before it.

This is multi-week scope. Treat each phase above as its own implementation pass (explore current code → implement → verify against that phase's checklist below) rather than attempting all of it in one sitting.

## Verification per phase

- **Phase 2**: register a real user through the UI (not curl) → OTP arrives in Mailpit → verify → login → OTP → land on `/app` authenticated; refresh the page and confirm the session survives via the refresh cookie.
- **Phase 3**: create nested pages via the UI, drag-reorder/reparent them, refresh and confirm persistence in Postgres.
- **Phase 4**: type rich content with an inline image, refresh, confirm both the Yjs state and the image (in MinIO) persisted.
- **Phase 5**: open the same page in two browser sessions (or two browsers), confirm edits and cursors sync live; kill one client and confirm the doc still persists correctly.
- **Phase 6**: share a page with a second test user at viewer role, confirm read-only access; confirm a page-level grant overrides an inherited workspace role; test a public link grant unauthenticated.
- **Phase 7**: export a page with an embedded image to PDF/DOCX/MD, open each file, confirm content and image fidelity.
- **Phase 8**: search returns the right pages; archive+restore round-trips; rollback restores a prior version; hit a rate-limited endpoint enough times to get throttled; `cargo test` passes.

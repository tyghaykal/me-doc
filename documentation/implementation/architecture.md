# me-doc — Architecture & Implementation Plan

## Context

`me-doc` is a self-hosted documentation web app in the spirit of Confluence/Notion: workspaces containing a nested tree of pages, a WYSIWYG editor with inline images, per-page/per-workspace sharing, and export to PDF/Markdown/Word.

Stack (fixed):
- Backend: Rust
- Frontend: Nuxt 4
- Database: PostgreSQL
- Cache/ephemeral store: Redis
- Object storage: MinIO (S3-compatible, dev only)
- Dev email catcher: Mailpit
- Orchestration: Docker / docker-compose

Auth: register + email/password login with **OTP as a true second factor** (password first, then a one-time code emailed via Mailpit in dev completes login). Editing is **real-time multi-user collaborative** (Google-Docs style), not just autosave.

## Architecture

### Services (docker-compose)
- `postgres` — primary datastore
- `redis` — OTP codes (TTL-native), refresh-token/session revocation, collab presence pub/sub
- `minio` + a one-shot `createbuckets` init container — S3-compatible storage for images/attachments/export output (dev only; prod would point at real S3 via env config)
- `mailpit` — SMTP catcher + web UI so OTP/verification emails are visible in dev
- `backend` — Rust API + WebSocket collab endpoint
- `frontend` — Nuxt 4 app

### Backend (Rust)
- **Framework**: `axum` (async, first-class WebSocket support via `tower`)
- **DB access**: `sqlx` against PostgreSQL, with `sqlx migrate` for schema migrations
- **Auth**: `argon2` password hashing, `jsonwebtoken` for access/refresh JWTs, refresh tokens tracked in Postgres (revocable), OTP codes generated and stored in Redis with short TTL and rate-limited resend
- **Email**: `lettre` sending SMTP to Mailpit in dev (swap SMTP host/creds for prod)
- **Object storage**: `aws-sdk-s3` pointed at MinIO's S3-compatible endpoint; presigned PUT URLs for image/file uploads from the editor
- **Real-time collaboration**: `yrs` (Rust port of Yjs) driving a CRDT doc per page; `axum` WebSocket handler at `/ws/pages/:id` broadcasts updates between connected clients (protocol-compatible with a `y-websocket`-style provider on the frontend) and periodically persists snapshots to Postgres
- **Export**:
  - Markdown: custom serializer directly from the Tiptap/ProseMirror JSON document — no external deps
  - Word (.docx): `docx-rs`, mapping the document JSON (headings, paragraphs, lists, images) to docx-rs builder calls
  - PDF: server renders the page to HTML, then `chromiumoxide` (headless Chromium) prints it to PDF — gives real WYSIWYG fidelity that pure-Rust PDF libraries can't match for rich text + images
- **Permissions model**: workspace roles (`owner`/`admin`/`member`/`guest`) plus a `permissions` table keyed by subject (`workspace` or `page`) and principal (`user` or public `link` token) with role `viewer`/`editor`. Resolution walks up the page tree (page → parent → … → workspace) and stops at the first explicit override, otherwise inherits.

### Data model (Postgres, high level)
- `users` (id, email, password_hash, email_verified_at, created_at)
- `workspaces` (id, name, slug, owner_id, created_at)
- `workspace_members` (workspace_id, user_id, role)
- `pages` (id, workspace_id, parent_page_id nullable, title, slug, order_index, archived_at, created_by, timestamps)
- `page_content` (page_id, yjs_state bytea, plain_text tsvector for search, updated_at) — live doc state
- `page_versions` (id, page_id, snapshot, version_no, created_at) — history for rollback
- `permissions` (id, subject_type, subject_id, principal_type, principal_id nullable, role, link_token nullable, expires_at nullable)
- `attachments` (id, workspace_id, page_id nullable, s3_key, filename, mime_type, size, uploaded_by, created_at)
- `refresh_tokens` (id, user_id, token_hash, expires_at, user_agent, ip, revoked_at)

OTP codes live in Redis only (ephemeral, TTL-based), not in Postgres.

### Frontend (Nuxt 4)
- **Editor**: Tiptap (ProseMirror-based Vue bindings) — has an official Yjs collaboration extension plus image node support
- **Realtime sync**: Yjs client + a small custom WebSocket provider talking to the Rust `/ws/pages/:id` endpoint; shows live cursors/presence of other editors
- **Images**: paste/drag-drop uploads via presigned MinIO URL, inserted as an image node in the doc
- **Pages**: `/register`, `/verify-otp`, `/login`, `/login/otp`, workspace dashboard with a collapsible page-tree sidebar (drag-and-drop reorder/reparent), per-page share dialog (invite by email + role, or generate public link), export menu (PDF/MD/DOCX)
- **State**: Pinia for client state; calls the Rust API directly (CORS-enabled) rather than proxying through Nuxt server routes, since this is an authenticated app rather than a public marketing site
- **Styling**: Tailwind CSS v4 via the official `@tailwindcss/vite` plugin (Tailwind Labs' first-party Vite integration — no PostCSS config, no `tailwind.config.js`; just the Vite plugin plus a single `@import "tailwindcss";` CSS entry file). The community `@nuxtjs/tailwindcss` module was tried first but pulls in an incompatible Nuxt-3-era `@nuxt/kit`/jiti dependency chain that breaks under Nuxt 4
- **Rendering**: landing page (`/`) uses Nuxt's default universal (SSR) rendering; the authenticated app (`/app/**`) is forced to CSR via a `routeRules` entry in `nuxt.config.ts` (`{ '/app/**': { ssr: false } }`), matching the direct-to-API client-only architecture above

### Repo layout
```
me-doc/
  documentation/               (this plan + future design docs)
  docker-compose.yml
  docker-compose.override.yml  (dev hot-reload volumes)
  .env.example
  backend/
    Cargo.toml
    migrations/
    src/{main.rs, config.rs, auth/, workspaces/, pages/, sharing/, collab/, export/, storage/, email/, db/}
  frontend/
    nuxt.config.ts
    app/{pages/, components/, composables/, stores/}
```

## Build phases

1. **Scaffolding** — docker-compose with all infra services healthy (postgres, redis, minio+bucket-init, mailpit); backend skeleton with a `/health` endpoint hitting Postgres/Redis; frontend skeleton hitting backend `/health`.
2. **Auth** — register, email OTP verification, login (password → OTP → JWT), refresh-token rotation, default workspace auto-created on register.
3. **Workspaces & pages** — CRUD + tree structure/ordering APIs, sidebar tree UI with drag-and-drop.
4. **Editor (static first)** — Tiptap WYSIWYG wired to `page_content`, image upload to MinIO, manual save.
5. **Realtime collaboration** — `yrs` + WebSocket sync server, Yjs provider on the frontend, presence/cursors, periodic snapshot persistence.
6. **Sharing & permissions** — roles, per-page share dialog, public link tokens, permission-resolution middleware enforced on every read/write route.
7. **Export** — Markdown (native serializer), DOCX (`docx-rs`), PDF (`chromiumoxide` HTML render).
8. **Polish** — full-text search (Postgres `tsvector`), trash/archive + restore, page version history/rollback, rate limiting, basic test coverage.

## Verification per phase
- Phase 1: `docker compose up` brings up all services healthy; backend `/health` returns DB+Redis OK; frontend loads and shows backend status.
- Phase 2: register a user → OTP email appears in Mailpit UI → verify → login with password → second OTP → receive JWT; default workspace exists in Postgres.
- Phase 3–4: create nested pages via UI, reorder them, type rich content with an inline image, refresh and confirm persistence.
- Phase 5: open the same page in two browser sessions, confirm edits sync live.
- Phase 6: share a page with a second test user at viewer role, confirm they can read but not edit; confirm workspace-level permission is overridden correctly by a page-level grant.
- Phase 7: export a page with an embedded image to PDF/DOCX/MD and open each output file to confirm content and image fidelity.

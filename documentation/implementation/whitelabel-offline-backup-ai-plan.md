# White-label branding, offline/local mode, DB backup cron, AI slash commands

Companion to `tasks.md` — same work, tracked there as Phase 17 (#108-#111).

## Context

Four independent features requested together:
1. Product name becomes a deploy-time setting (`PRODUCT_NAME`, default `MeDoc`) instead of hardcoded "me-doc".
2. A local-only editing mode: open/save a document straight to the user's filesystem, no server page involved — Share is unavailable there because there's no server-side page/permission record to share.
3. A scheduled job that `pg_dump`s Postgres, gzips it, and uploads to the existing `S3_BUCKET_BACKUP`.
4. AI actions (rephrase / fix grammar / reformat / proofread / explain) added to the existing `/` slash-command menu, calling whatever OpenAI-chat-completions-compatible endpoint each user configures with their own API URL + key + model (BYOK — works with OpenAI, OpenRouter, Groq, self-hosted Ollama's OpenAI-compat route, etc).

Not in scope: multi-tenant white-labeling (logo/colors), syncing a local doc back into a server workspace, backup restore tooling, or a chat-style AI panel — only the five fixed slash-menu actions were asked for.

## 1. White-label product name

- `backend/src/config.rs`: add `product_name: String`, `env::var("PRODUCT_NAME").unwrap_or_else(|_| "MeDoc".into())`.
- `backend/src/email/mod.rs`: replace every hardcoded `me-doc` string (subject lines, HTML template header/footer) with `state.config.product_name` / a parameter threaded through the existing template functions.
- `frontend/nuxt.config.ts`: read `process.env.NUXT_PUBLIC_PRODUCT_NAME` (build-time) for `app.head.title`/meta description, and add `public.productName` to `runtimeConfig` for client-side use (e.g. the settings/local-mode UI, any "New in {name}" copy).
- `frontend/app/components/landing/LandingNav.vue`, `LandingFooter.vue`: replace literal `me-doc` with `useRuntimeConfig().public.productName`.
- `docker-compose.yml`: pass `PRODUCT_NAME` to `backend`, `NUXT_PUBLIC_PRODUCT_NAME: ${PRODUCT_NAME:-MeDoc}` to `frontend`.
- `.env` / `.env.example`: add `PRODUCT_NAME=MeDoc`.
- Leave `CommentSidebar.vue`'s `me-doc.comments-sidebar-width` localStorage key and `storage/mod.rs`'s `me-doc-static` S3 prefix alone — internal identifiers, not user-facing branding.

## 2. Offline / local mode

- New route `frontend/app/pages/app/local.vue` (under the existing `/app/**` → `ssr: false` rule, no `nuxt.config.ts` change needed).
- File format: self-contained `.html` (Tiptap `editor.getHTML()` / `setContent()`) — no new markdown-serializer dependency needed (the installed `marked` package only parses markdown → HTML, used today for `.md` import; going the other direction would need a new dep, which the feature doesn't need if the local format is just HTML).
- Filesystem access: the native File System Access API (`window.showOpenFilePicker` / `showSaveFilePicker`), feature-detected via `'showOpenFilePicker' in window`. Fallback for Firefox/Safari (no support): `<input type="file">` for open, an `<a download>` Blob link for save — same editor, degraded file flow.
- Editor reuse: `Editor.vue` is tightly coupled to the Yjs collab room. Extract its non-collab extension list (StarterKit, Table, Image, Highlight, TaskList/TaskItem, the custom Diagram node, SlashCommand, etc.) into a small shared composable so a new lightweight local editor and the existing collaborative one both build from it; the collaborative one then layers `Collaboration`/`CollaborationCaret` on top. No `Y.Doc`, no websocket, no collab room for local mode.
- Local page gets its own minimal toolbar (New / Open / Save / Save As / Export) — it never renders `AppTopbar.vue`'s Share button, so "share disabled in offline mode" falls out of the route being a separate component tree rather than a flag threaded through the collaborative page's UI.
- Entry point: a "Local documents" link in `AppSidebar.vue` pointing at `/app/local`.

## 3. Postgres backup → gzip → S3, on a cron schedule

- `scripts/backup-db.sh`: `pg_dump "$DATABASE_URL" | gzip` → `/tmp/<db>-<UTC timestamp>.sql.gz` → `mc cp` to `local/$S3_BUCKET_BACKUP/backups/...`; if `BACKUP_RETENTION_DAYS` is set, prune with `mc rm --recursive --force --older-than "${BACKUP_RETENTION_DAYS}d" local/$S3_BUCKET_BACKUP/backups/`.
- `scripts/backup/Dockerfile`: `FROM postgres:16-alpine` (already has `pg_dump`), add `dcron` + the static `mc` binary, entrypoint writes `$BACKUP_CRON_SCHEDULE` into a crontab file and runs `crond -f`.
- `docker-compose.yml`: new `db-backup` service (build `./scripts/backup`), `env_file: .env`, environment reusing `POSTGRES_*`/`S3_*`/`S3_BUCKET_BACKUP` plus the two new vars below, `depends_on: postgres (healthy), minio (healthy)`, `restart: unless-stopped`.
- `minio-createbuckets` (or `scripts/init-minio.sh`): also `mc mb -p local/$S3_BUCKET_BACKUP`, so the backup bucket exists on first `docker compose up` the same way the primary bucket does.
- `.env` / `.env.example`: add `BACKUP_CRON_SCHEDULE=0 3 * * *` and `BACKUP_RETENTION_DAYS=30` (`S3_BUCKET_BACKUP` already exists in both files).

## 4. AI slash-command actions (BYOK)

- Migration `backend/migrations/0015_ai_settings.sql`: `user_ai_settings(user_id pk/fk → users, api_url text, api_key_encrypted bytea, api_key_nonce bytea, model text, updated_at)`.
- New dependency `aes-gcm` (small, pure-Rust AEAD; nothing already in `Cargo.toml` does authenticated encryption) — API keys are a real secret, so they're encrypted at rest, not stored plaintext. New required env var `AI_ENCRYPTION_KEY` (32-byte, base64) in `config.rs`, `.env`/`.env.example`.
- New `backend/src/ai/mod.rs`:
  - `GET /ai/settings` → `{ api_url, model, has_key }` — key itself never returned.
  - `PUT /ai/settings` → upserts `api_url`/`model`, re-encrypts `api_key` only when a new one is sent (blank/omitted leaves the stored key untouched).
  - `POST /ai/complete` `{ instruction: rephrase|fix_grammar|reformat|proofread|explain, text }` → loads + decrypts the caller's settings, calls their configured endpoint's `/chat/completions` via `reqwest` (already a dependency) with a small fixed system prompt per instruction, returns `{ result }`. `AuthError::Validation` if no settings/key configured yet.
  - Registered in `lib.rs` (`pub mod ai;`, `.merge(ai::router())`).
- `frontend/app/pages/app/settings.vue`: form for API URL / API key / model id, backed by `GET`/`PUT /ai/settings`.
- `frontend/app/components/slash-command.ts`: new "AI" group — Rephrase, Fix grammar, Reformat, Proofread, Explain. Each takes the current block's text (the block the `/` was typed in — there's no text selection at that point, the trigger already collapsed it), calls `POST /ai/complete`; the first four replace the block's content with the result, Explain inserts the explanation as a new block below. A "configure your API key" prompt (linking to `/app/settings`) shows on a "not configured" error instead of a raw failure.
- Entry point to settings: a link from the user/account menu (wherever `AppTopbar.vue`/`AppSidebar.vue` currently exposes account actions).

## Critical files
- Backend: `backend/src/config.rs`, `email/mod.rs`, `lib.rs`, new `ai/mod.rs`, new migration, `Cargo.toml`.
- Frontend: `nuxt.config.ts`, `components/landing/LandingNav.vue`/`LandingFooter.vue`, `components/Editor.vue`, new `pages/app/local.vue`, new `pages/app/settings.vue`, `components/slash-command.ts`, `components/AppSidebar.vue`.
- Infra: `docker-compose.yml`, `.env`, `.env.example`, new `scripts/backup/Dockerfile`, new `scripts/backup-db.sh`, `scripts/init-minio.sh`.

## Verification
Manual against the running docker-compose stack (no test suite in this repo):
- Branding: set `PRODUCT_NAME=Acme Docs`, rebuild, confirm it shows in the browser tab title, landing nav/footer, and a verification email.
- Local mode: open `/app/local`, create content, Save As to a real file, reload the browser, Open that file back, confirm content round-trips and no Share control is present anywhere on the page.
- Backup: exec into `db-backup`, run the script manually, confirm a `.sql.gz` lands in MinIO's backup bucket and restores cleanly with `gunzip | psql`.
- AI: configure a real (or local Ollama) OpenAI-compatible endpoint in Settings, run each of the five slash actions against sample text, confirm the block updates (or, for Explain, a new block appears) and that clearing the key makes the menu prompt to configure instead of erroring raw.

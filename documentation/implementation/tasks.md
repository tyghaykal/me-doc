# me-doc — Implementation Task Checklist

Companion to `full-implementation-plan.md` — same work, broken into trackable tasks. Task numbers match the ones created in the session task tracker (`TaskList`/`TaskCreate`); update this file's checkboxes as you complete each one there, since the tracker itself doesn't persist across sessions.

`blocked by` means: don't start until those task numbers are done — either a hard code dependency, or the earlier task establishes a pattern the later one builds on.

## Cross-cutting

- [x] **#1** Bump frontend Dockerfiles to `node:22-slim`

## Phase 2: Auth frontend

- [x] **#2** Add Pinia dependency + module config
- [x] **#3** Build auth Pinia store — *blocked by #2*
- [x] **#4** Build `useApi` composable with 401 refresh-retry — *blocked by #3*
- [x] **#5** Build auth pages (register/verify-otp/login/login-otp) — *blocked by #3, #4*
- [x] **#6** Build auth route guard middleware — *blocked by #3*

## Phase 3: Workspaces & pages

- [x] **#7** Migration — pages table
- [x] **#8** Backend `AuthenticatedUser` extractor
- [x] **#9** Backend workspaces router — *blocked by #8*
- [x] **#10** Backend pages CRUD + tree endpoints — *blocked by #7, #8*
- [x] **#11** Frontend pages Pinia store — *blocked by #10*
- [x] **#12** Frontend `PageTree` component with drag-and-drop — *blocked by #11*
- [x] **#13** Frontend workspace dashboard page — *blocked by #12, #5, #6*

## Phase 4: Editor (static first)

- [x] **#14** Migrations — page_content + attachments tables
- [x] **#15** Backend storage module (S3 presign)
- [x] **#16** Backend page content + attachment endpoints — *blocked by #14, #15*
- [x] **#17** Frontend Tiptap editor component — *blocked by #16, #13*

## Phase 5: Realtime collaboration

- [x] **#18** Backend yrs BroadcastGroup-equivalent doc registry (`DocRegistry`/`DocRoom` in `collab/mod.rs`, hand-rolled since `yrs-axum` pins incompatible `yrs`/`axum` versions) — *blocked by #17*
- [x] **#19** Backend `/ws/pages/:id` WebSocket route — *blocked by #18*
- [x] **#20** Backend persistence + eviction logic (5s debounced flush, final persist + DashMap eviction on last-client-disconnect) — *blocked by #19*
- [x] **#21** Frontend Yjs WebSocket provider + collab cursors (`y-websocket` `WebsocketProvider` bound to the editor's shared `Y.Doc`, `@tiptap/extension-collaboration-caret` for remote cursors — v3's caret extension replaces v2's `collaboration-cursor`) — *blocked by #19, #17*

## Phase 6: Sharing & permissions

- [x] **#22** Migration — permissions table
- [x] **#23** Backend permission resolution CTE + extractor — *blocked by #22*
- [x] **#24** Backend share/link endpoints + retrofit existing routes — *blocked by #23*
- [x] **#25** Frontend `ShareDialog` component — *blocked by #24*

## Phase 7: Export

- [x] **#26** Backend Markdown export serializer (`export/mod.rs`, Yjs XML fragment → Markdown, unit-tested) — *blocked by #17*
- [x] **#27** Backend DOCX export (`export/docx.rs` via `docx-rs`, fed by re-parsing the exporter's own Markdown output with `pulldown-cmark` rather than a second Yjs tree walk) — *blocked by #26*
- [x] **#28** Backend PDF export (`export/pdf.rs` via `genpdf`, same Markdown-intermediate approach; relies on `fonts-liberation`, already installed in both backend Dockerfiles) — *blocked by #26*
- [x] **#29** Backend export route + frontend export menu (`format=md|docx|pdf` query param; `ExportMenu.vue` dropdown wired into the workspace header) — *blocked by #26, #27, #28, #24*

## Phase 8: Polish

- [x] **#30** Full-text search — *blocked by #16*
- [x] **#31** Trash/restore — *blocked by #10*
- [x] **#32** Version history/rollback (`page_versions` table + `versions/mod.rs`; snapshotted once per finished editing session via `collab::persist_version`, dedup'd on identical bytes) — *blocked by #20*
- [x] **#33** Rate limiting
- [x] **#34** Initial test coverage — *blocked by #23*

## Bugs found and fixed during live verification (not separate tasks, fixed inline)

- S3 client panicked at runtime: `aws-sdk-s3`/`aws-config` require an explicit `behavior_version` — added `.behavior_version(BehaviorVersion::latest())`.
- Rate limiter (`tower_governor`) 500'd every request: its default IP-key extractor needs `ConnectInfo<SocketAddr>`, which requires serving via `into_make_service_with_connect_info::<SocketAddr>()` instead of a bare `Router` — fixed in `main.rs`.
- Presigned attachment upload URLs were signed against the internal Docker hostname (`minio:9000`), unreachable from the browser — added a second S3 client (`s3_presign`, `AppState`) built against a new `S3_PUBLIC_ENDPOINT` config value (defaults to `http://localhost:9010`), used only for presigning. Verified end-to-end: presign → PUT upload → public GET all return 200.
- Frontend dev server (in the `frontend-1` container) got wedged (empty replies on all routes) after many rapid concurrent HMR reloads from parallel agents — recovered with `docker compose restart frontend`.
- `backend/src/export/mod.rs` failed to compile once Docker was back: it used `yrs::XmlNode`/`yrs::Value`, which don't exist in the actually-resolved `yrs 0.27.3` (confirmed by reading the crate source in the registry cache — these were renamed to `XmlOut`/`Out` in that version). Fixed all call sites to the real names.

## Status

All 34 tasks are implemented, verified live, and the known-debt item below is resolved. Docker is back up; the full stack was rebuilt (`docker compose up -d --build`) and both `backend` and `frontend` came up healthy with no compile errors (after the `XmlOut`/`Out` fix above). The `page_versions` migration applied cleanly.

End-to-end smoke test performed against the live stack (register → OTP via Mailpit → verify → real Yjs client over the collab WebSocket → export → version history):
- Realtime collab (#18-#20): a Node `y-websocket` client connected to `/ws/pages/:id`, synced, wrote a heading/paragraph/bold-text/bullet-list into the shared doc, and disconnected — the server persisted `page_content.yjs_state` (278 bytes) and recorded a `page_versions` row on disconnect, confirmed via direct DB query.
- Export (#26-#29): `GET /pages/:id/export?format=md|docx|pdf` all returned 200 with correct content types; the Markdown matched the edits made (`## Hello E2E`, bold text, bullet list); the DOCX is a well-formed zip with `word/document.xml`; the PDF has a valid `%PDF-1.3` header and `%%EOF` trailer.
- Version history (#32): `GET /pages/:id/versions` listed the one recorded version; `POST /pages/:id/versions/:version_id/restore` returned 200.
- Frontend routes `/`, `/app`, `/register`, `/login` all return 200 (frontend #21's `y-websocket`/`collaboration-caret` deps installed clean, confirmed via `pnpm install` log during the rebuild — no in-browser cursor test performed, that needs two real browser tabs).

## Resolved: lib.rs/main.rs duplication

`main.rs` now reuses `me_doc_backend`'s module tree and `AppState` via `use me_doc_backend::{...}` instead of duplicating the module declarations and struct definition — the bin crate compiles once, off of the lib crate, dropping the earlier double-compile debt.

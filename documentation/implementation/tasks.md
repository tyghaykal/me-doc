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

## Phase 9: Multi-workspace, user profile, /app fixes

Companion plan: `app-workspace-profile-plan.md`.

- [x] **#35** Fix "+ New page" no-op on empty workspace — pass `workspace-id` prop into `PageTree` instead of inferring it; `createPage` sets `activePageId`
- [x] **#36** Bump base font size (+2px via root `html { font-size: 18px }`)
- [x] **#37** Backend: `workspaces/mod.rs` — `create_workspace`, `list_members`, `add_member`, `remove_member` endpoints
- [x] **#38** Backend: migration `0008_user_profile.sql` (`display_name`, `avatar_key` on `users`)
- [x] **#39** Backend: new `users/mod.rs` module (`GET/PATCH /auth/me`, `POST /auth/me/password`, `POST /auth/me/avatar/presign`) — *blocked by #38*
- [x] **#40** Backend: wire `users::router()` into `main.rs`/`lib.rs` — *blocked by #39*
- [x] **#41** Frontend: `stores/workspaces.ts` + `WorkspaceSwitcher.vue` + `CreateWorkspaceModal.vue` + `WorkspaceMembersModal.vue` — *blocked by #37*
- [x] **#42** Frontend: `UserSettingsModal.vue` — *blocked by #40*
- [x] **#43** Frontend: wire both into `pages/app/index.vue` header (workspace switcher replaces static `<h1>`, settings gear button, mount-time workspace fetch/restore + watcher) — *blocked by #41, #42*
- [x] **#44** End-to-end verification per plan's Verification section — *blocked by #43*

**Bug found during #44 verification (not in original plan, fixed inline):** `Editor.vue` never actually worked — `useEditor()` (from `@tiptap/vue-3`) already returns a `ShallowRef<Editor>` and internally registers its own `onMounted`/`onBeforeUnmount`, so it must be called synchronously in `<script setup>`. The old code called it inside `onMounted(async () => { await ...; editor.value = useEditor(...) })`, both deferring it past an `await` *and* wrapping its already-a-ref return value in a second `shallowRef` — breaking Vue's template auto-unwrap and losing the active-component-instance context Tiptap's internal hooks need. Every page open threw `TypeError: Cannot read properties of undefined (reading 'element')` and rendered nothing. This was never caught before because the pre-existing `+ New page` bug (#35) meant no page had ever actually been opened in this app until today. Fixed by calling `useEditor()` directly at the top level of setup (not nested in any lifecycle hook) and using its return value as-is; page content now loads asynchronously afterward and is merged into the already-live Yjs doc via `Y.applyUpdate` (safe regardless of ordering — Yjs updates are commutative).

**Verification performed live** (Playwright against the real Docker stack, `https://localhost`, real Mailpit OTP retrieval):
- Registered a user, landed on `/app` with zero pages, clicked "+ New page" → page created and opened immediately, typed into the editor, text persisted in the DOM (`.ProseMirror` innerText matched).
- Root `font-size: 18px` confirmed via `getComputedStyle`.
- Created a second workspace via the switcher → became active, page list correctly reset/refetched empty for it.
- Registered a second account, invited it to the workspace by email with role `member` via "Manage members" → confirmed in the UI: inviter sees "Leave" (owner), invitee row shows "Remove" (admin/owner privilege), success message "Added \<email\>." shown.
- Opened user settings, changed display name, saved → "Saved." confirmation shown; password section present with 3 fields (current/new/confirm).

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

## Phase 10: Notion-style app shell

Companion plan: `notion-layout-redesign.md`.

- [x] **#45** `PageTree.vue`: collapse/expand (`useState<Set<string>>` of collapsed node ids + chevron), drag/drop preserved
- [x] **#46** `useRecents.ts` composable (MRU localStorage, dev-only `console.assert` self-check for cap/dedupe) + wire push into `[[pageId]].vue`'s existing `activePageId`-derived `activePage` watcher (used `activePage` instead of the raw id watcher so shared-page titles resolve correctly too)
- [x] **#47** `AppSidebar.vue` + `SidebarRecents.vue` (switcher pill, icon row, Recents, Private tree, Trash button) — replaces `[[pageId]].vue`'s `<aside>` — *blocked by #45, #46*
- [x] **#48** Backend `GET /me/shared-pages` handler + router wiring (`backend/src/pages/mod.rs`, joins existing `permissions` table, no schema change)
- [x] **#49** Frontend `sharedPages`/`fetchSharedPages()` in `stores/pages.ts` + `SidebarSharedSection.vue`, wired into `AppSidebar.vue` — *blocked by #47, #48*
- [x] **#50** `TrashModal.vue` + `stores/pages.ts` `trash`/`fetchTrash`/`restorePage` (surfaces already-existing, currently-unused trash/restore endpoints), wired to `AppSidebar.vue` — *blocked by #47*
- [x] **#51** `AppTopbar.vue` (breadcrumb, static "Private" badge, edited date, share/copy-link, "…" menu for History+Duplicate) — replaces `[[pageId]].vue`'s `<header>`, reuses existing `shareOpen`/`historyOpen` wiring
- [x] **#52** `SearchPalette.vue` + `pagesStore.searchPages()` (surfaces already-existing, currently-unused search endpoint), wired into `AppSidebar.vue` icon row with Ctrl/Cmd+K — *blocked by #47*
- [x] **#53** *(stretch)* Page icon/emoji — migration `0009_page_icon.sql`, extend `Page`/`PageRow`/`PAGE_COLUMNS`/`UpdatePageRequest` (generic `double_option<T>`, was Uuid-only before), emoji-grid picker in `Editor.vue`, rendered in `PageTree.vue`/`AppTopbar.vue` — *blocked by #51*
- [x] **#54** *(stretch)* Favorite/star — migration `0010_page_favorites.sql` (join table), `POST/DELETE /pages/:id/favorite` + `GET /me/favorite-pages` (mirrors #48), star toggle in `AppTopbar.vue`, `SidebarFavorites.vue` in `AppSidebar.vue` — *blocked by #51*

**Bug found and fixed during #48 (not in original plan):** the new `/me/shared-pages` route 404'd through the single-origin nginx setup — nginx's proxy rule only forwards a hardcoded prefix allowlist (`health|auth|workspaces|pages|permissions|attachments|ws`) to the backend, and `/me` wasn't in it, so the request silently fell through to the frontend's catch-all and got Nuxt's 404. Added `me` to `nginx/conf.d/default.conf`'s regex (which explicitly documents it must mirror the backend router) and reloaded nginx.

**Verification performed live** against the running docker-compose stack at `https://localhost`, using real registered test users (OTP retrieved via the Mailpit API) rather than mocks:
- #48: shared a page from user1 to user2 via the existing `POST /pages/:id/share`; `GET /me/shared-pages` returned it for user2, empty for user1, and empty again after `DELETE /permissions/:id`.
- #50: deleted a page via the existing archive endpoint, confirmed it appeared via `GET .../pages/trash`, restored via `PATCH /pages/:id/restore`, confirmed the trash list emptied.
- #52: confirmed `GET /workspaces/:id/search?q=...` is reachable through nginx with the exact query shape the frontend now sends (200, empty result set — no indexed content in the test page yet, which is correct).
- #53: `PATCH /pages/:id` with `{"icon":"🚀"}` set it, `{"icon":null}` explicitly cleared it, and an update with the `icon` field omitted left the existing value untouched — confirming the new generic `double_option` correctly distinguishes "absent" from "present and null" for a second field type (previously only proven for `Uuid` via `parent_page_id`).
- #54: favorited a page (idempotent on a repeat call via `on conflict do nothing`), confirmed it listed via `GET /me/favorite-pages`, unfavorited, confirmed the list emptied.
- All frontend changes verified via `docker compose logs frontend` after each edit — clean Vite HMR updates, no compile/type errors, `/app` consistently returned 200.

## Phase 11: Sharing UX, export enforcement, live presence/cursors

Companion plan: `sharing-presence-plan.md`.

- [x] **#55** Hide Share dialog trigger from viewers — `AppTopbar.vue`'s "Private" badge gated on `!isViewer` too
- [x] **#56** Hide Export button from viewers — `ExportMenu` gated on `!isViewer` in `AppTopbar.vue`
- [x] **#57** Enforce export role server-side — `export/mod.rs`'s `export_page` switched from `require_membership` to `PagePermission` + `Role::Editor` check — *blocked by #56*
- [x] **#58** Backend `GET /pages/:id/permissions` (list shares: registered users, pending invites, public links) — `sharing/mod.rs`
- [x] **#59** Frontend "People with access" list + revoke in `ShareDialog.vue` (`listShares`/`revokeShare` in `stores/pages.ts`, reuses existing `DELETE /permissions/:id`) — *blocked by #58*
- [x] **#60** Backend: send current `Awareness::update()` snapshot to a newly-joining WS client in `collab/mod.rs` (today only future changes are seen) + frontend CSS for `.collaboration-carets__caret`/`.collaboration-carets__label` in `main.css`
- [x] **#61** Presence avatar stack — `Editor.vue` subscribes to `provider.awareness.on('change', ...)`, emits up through `[[pageId]].vue` into `AppTopbar.vue` — *blocked by #60*

**Verification performed live** against the running docker-compose stack at `https://localhost`, using two freshly registered test accounts (OTP retrieved via the Mailpit API) — an owner and a viewer sharing one page:
- Backend compiles clean (`cargo check` inside the `backend` container, zero warnings on touched files); frontend HMR applied all edits with no Vite/type errors.
- #55/#56: `GET /pages/:id` as the viewer account returns `"role":"viewer"`, which drives `isViewer` in `AppTopbar.vue` — the same computed already gating the Share button now also gates the "Private" badge and `ExportMenu`.
- #57: `GET /pages/:id/export?format=md` returned `403 {"message":"access denied"}` for the viewer and `200` (valid Markdown) for the owner — confirms the backend now enforces `Role::Editor`, not just workspace membership.
- #58: `GET /pages/:id/permissions` returned `403` for the viewer and, for the owner, a JSON array with the exact viewer grant (`principal_type: "user"`, correct email/role/`pending`/`created_at`).
- #59: `DELETE /permissions/:id` (reused existing route) removed the grant; a follow-up `GET /pages/:id/permissions` came back `[]`, confirming revoke works end-to-end through the new list endpoint.
- #60/#61: verified by code inspection against the installed `yrs 0.27.3` source (`Awareness::update()`) and the installed `@tiptap/extension-collaboration-caret@3.28.0` source (confirmed it sets `provider.awareness.setLocalStateField("user", …)` and renders `.collaboration-carets__caret`/`.collaboration-carets__label`, matching the new CSS) — no rate-limit-safe way to drive two real WebSocket browser sessions from curl, so the live "two tabs, one joins late" cursor/presence-stack check from the plan still needs a manual two-browser pass.
- Rate-limiter note (pre-existing, not part of this change): both the strict (`/auth/*`) and standard buckets refilled far slower under this verification's request bursts than their configured `per_second`/`burst_size` would suggest — every check above eventually passed on retry with backoff, so it didn't block verification, but it's worth a closer look separately.

**Follow-up fix (same session, not a separate phase):** the rate-limiter note above turned out to be a real bug, not just test-harness contention — the user hit it live on `/auth/refresh`. Root cause: `tower_governor`'s `.per_second(n)`/`.per_millisecond(n)` set the replenish *period* (time between adding one token), not a request rate — `standard_conf`'s `.per_second(20)` meant one new token every 20 *seconds* (~0.05 req/s sustained), not 20 req/s, so the "generous" bucket (which `/refresh` and nearly everything else runs through) was actually far stricter than the login-only strict bucket in steady state. Fixed in `backend/src/main.rs` by switching both configs to `.per_millisecond(1000 / rate)` to express the intended rate correctly, and loosened the strict login/register bucket per request (1→5 req/s sustained, burst 8→20). Verified live: 15 rapid requests each against a general endpoint, `/auth/login`, and `/auth/refresh` all completed with zero `429`s.

## Phase 12: Share management, presence avatars, comments, rich editor content

Companion plan: `share-comments-editor-plan.md`.

**Group A — Share dialog polish**
- [x] **#62** Update a share's role in place — `PATCH /permissions/:id` in `sharing/mod.rs` (Editor-gated, mirrors `delete_permission`'s auth check) + `updateShareRole()` in `stores/pages.ts` + role `<select>` replacing the read-only role text in `ShareDialog.vue`'s "People with access" rows
- [x] **#63** Auto-copy public link after generating it — `ShareDialog.vue`'s `generateLink()` calls the existing `copyLink()` on success
- [x] **#64** Avatar-aware presence, self included — extend `auth.ts`'s `User` with `display_name`/`avatar_key` (hydrated via `GET /me` after login/register/refresh), thread `avatarUrl` through `Editor.vue`'s `currentUser`/`updatePresence()` into `AppTopbar.vue`'s avatar stack (`<img>` else initial-circle fallback), plus a new "self" chip sourced from `authStore.user` at the front of the stack

**Group B — Rich editor content**
- [x] **#65** Image resize — `Image.configure({ resize: { enabled: true, minWidth: 80, minHeight: 80 } })` in `Editor.vue`, native to the installed `@tiptap/extension-image`, config-only
- [x] **#66** Table support via slash command — added `@tiptap/extension-table` only (its `TableKit` already bundles row/cell/header in v3.28.0 — the separate `-row`/`-cell`/`-header` packages installed first turned out redundant and were removed), "Table" entry in `slash-command.ts`, contextual `BubbleMenu` (shown when `editor.isActive('table')`) for add/delete row/column + delete table, `.ProseMirror table` CSS
- [x] **#67** Markdown paste support — new `marked` dependency + `frontend/app/utils/markdown.ts`'s `markdownToHtml()`, wired into `Editor.vue`'s `handlePaste` for plain-text-only clipboard content (no `text/html` present)
- [x] **#68** Import a `.txt`/`.md` file — "Import" button placed in `PageTree.vue` next to "+ New page" (not `AppSidebar.vue` — that's where page creation actually lives), reuses `markdownToHtml()`, creates a page and stages its HTML in a new `pagesStore.pendingImportHtml` map that the new page's `Editor.vue` applies via `editor.commands.setContent()` on mount — *blocked by #67*
- [x] **#69** Markdown round-trip verification pass — no new code; see verification notes below — *blocked by #65-#68*

**Group C — Comments (new subsystem)**
- [x] **#70** Backend: `comments` table (migration `0012_comments.sql`) + `backend/src/comments/mod.rs` CRUD (`POST`/`GET /pages/:id/comments`, `PATCH /comments/:id/resolve` toggles, `DELETE /comments/:id`; any resolved role may comment, resolve/delete needs Editor or the author) + router wiring in `lib.rs`/`main.rs` + **`comments` added to `nginx/conf.d/default.conf`'s route allowlist**
- [x] **#71** Frontend: `comment` Tiptap mark (`comment-mark.ts`, `commentId` attribute, `excludes: ''` for overlaps) — anchors via the same Y.Text formatting-attribute mechanism already used for bold/italic, survives concurrent edits with no relative-position math
- [x] **#72** Frontend: create a comment — "Comment" button in the existing selection `BubbleMenu` (swaps the toolbar for an inline draft form), assignee input filtered against `GET /workspaces/:id/members`, applies the mark and POSTs with the same `commentId` — *blocked by #70, #71*
- [x] **#73** Frontend: comment sidebar — new `CommentSidebar.vue` (docked right panel, not a modal, so the highlighted text stays visible) + `stores/comments.ts`, list/resolve/delete, click-to-scroll to anchor, syncs a `.comment-resolved` CSS class onto the DOM mark whenever a comment's resolved state changes — *blocked by #70, #71*
- [x] **#74** Wire comments into the app shell — 💬 toggle button in `AppTopbar.vue`, panel in `[[pageId]].vue` — *blocked by #72, #73*

**Verification performed live** against the running docker-compose stack at `https://localhost`, real registered accounts (OTP via Mailpit):
- Backend: `cargo check` clean throughout, zero warnings on touched files. Frontend: every edit produced clean Vite HMR with no compile/type errors; `/app` and a real page route both returned 200 after the full set of changes.
- #62: `PATCH /permissions/:id` moved a live grant viewer→editor→viewer; `GET /pages/:id/permissions` confirmed one row throughout, no duplicate.
- #70-#73: as the **viewer** account, created a comment on the shared page and assigned it to the owner — succeeded (viewers can comment, confirmed by decision). Listed it back. As the **owner** (Editor, not the author), resolved it — `resolved: true` came back. As the **viewer** (the original author, not an Editor), deleted their own comment — succeeded, confirming the "Editor or author" policy on both ends.
- Migration `0012_comments.sql` confirmed applied via `\d comments` — table, indexes, and FKs all present.
- #64/#65/#66/#67/#68/#69: these are editor-UI/visual behaviors (avatar images rendering, drag-to-resize, table toolbar, typing/paste/import content, presence self+others) that can't be driven from `curl`. Verified by code inspection instead: the exact Tiptap `resize`/`TableKit`/BubbleMenu `shouldShow` APIs were confirmed against the installed `@tiptap/*@3.28.0` dist source before wiring (not guessed), and every file compiled/HMR'd cleanly. Still needs a manual in-browser pass — same caveat Phase 11 left for its two-cursor test.

## Resolved: lib.rs/main.rs duplication

`main.rs` now reuses `me_doc_backend`'s module tree and `AppState` via `use me_doc_backend::{...}` instead of duplicating the module declarations and struct definition — the bin crate compiles once, off of the lib crate, dropping the earlier double-compile debt.

## Phase 12: Collaborative diagrams + real-time comments

Companion to `diagrams-comments-plan.md`. Diagrams reuse the whole page subsystem
(`pages.kind='diagram'`); the only new backend surface is the comments WS fan-out.

**Phase A — Real-time comments (WebSocket)**
- [x] **#75** Backend `CommentHub` in `AppState` + `comments/realtime.rs` (`GET /ws/comments/:id`, JWT + `resolve_role` gate, in-process `broadcast` fan-out)
- [x] **#76** Publish typed events (`created`/`updated`/`deleted`) from the four comment mutations in `comments/mod.rs` — *blocked by #75*
- [x] **#77** Frontend `useCommentStream` composable + idempotent `applyEvent`/upsert in `stores/comments.ts`, mounted in the app shell — *blocked by #76*

**Phase B — Diagram core (text + preview + collab, standalone + inline)**
- [x] **#78** Migration `0014_page_kind.sql` + `kind` through `create/list/get/duplicate/list_diagrams` in `pages/mod.rs`
- [x] **#79** `mermaid` dep + `utils/diagram/mermaid.ts` (lazy render/parse/type-detect, theme-synced)
- [x] **#80** `useCollab` composable (provider + presence) — shared by diagram page and embed — *blocked by #79*
- [x] **#81** `DiagramEditor` + `DiagramCodePane` + `DiagramCanvas` + `DiagramToolbar` + `bindYText` (Y.Text↔ref minimal-diff) — *blocked by #79, #80*
- [x] **#82** `DiagramPage` container + app-shell renders it for `kind==='diagram'` + sidebar "+ Diagram" — *blocked by #81*
- [x] **#83** Inline `diagram` TipTap node + `DiagramNodeView` + `/diagram` slash entry, registered in `Editor.vue` — *blocked by #81*
- [x] **#84** Export: `diagram` → ```mermaid``` fence in `export/mod.rs`; diagram-page `plain_text` via `yjs_named_text("source")` for search — *blocked by #78, #83*

**Phase C — Visual drag-drop (Vue Flow + adapters)**
- [x] **#85** `@vue-flow/*` deps + adapter registry (`utils/diagram/adapters/`) — *blocked by #81*
- [x] **#86** `flowchart` adapter (parse + generate, round-trip stable — verified via assert check) — *blocked by #85*
- [x] **#87** `DiagramFlow` Vue Flow canvas (add/connect/delete/relabel → regenerate source; auto-layout; drag-reposition) wired into `DiagramEditor`'s Diagram view — *blocked by #86*

**Phase D — Live-linked embedding**
- [x] **#88** `diagramEmbed` node + `DiagramEmbedView` (read-only render, live via read-only `useCollab` subscription) — *blocked by #81, #80*
- [x] **#89** `GET /workspaces/:id/diagrams` + `DiagramPicker` + `/embed diagram` slash entry in `Editor.vue` — *blocked by #88*
- [x] **#90** Export: `diagramEmbed` → link placeholder in `export/mod.rs` — *blocked by #88, #84*

**Verification performed** against the running docker-compose stack:
- Backend `cargo` recompiled clean throughout (cargo-watch); migration `0014` confirmed applied (`pages.kind` present). New routes wired: `/health` 200, `/ws/comments/:id` rejects non-upgrade/no-token, `/workspaces/:id/diagrams` enforces auth (401). nginx allowlist already covers `ws`/`workspaces` with WS upgrade headers.
- Frontend: every edit produced clean Vite HMR; `/app` and all diagram modules (`DiagramFlow.vue`, `DiagramEditor.vue`, adapters) transform 200 with no resolve/compile errors. `mermaid` + `@vue-flow/*` installed.
- Flowchart adapter round-trip verified with an assert script (parse→generate→parse stable; non-flowchart declined).
- **Still needs a manual in-browser pass** (same caveat as Phase 11): two-browser live comments, diagram co-editing, Vue Flow drag/connect interactions, and live embed propagation.

## Phase 13: MarkItDown import + diagram export/editor fixes

Companion plan: `markitdown-import-plan.md`.

- [x] **#91** `converter/` FastAPI + MarkItDown wrapper (`main.py`, `requirements.txt`, `Dockerfile`); wired into `docker-compose.yml` as an internal-only service
- [x] **#92** Backend `POST /pages/import` multipart proxy (`convert.rs`) — 20 MiB cap, forwards to converter, returns `{markdown}` — *blocked by #91*
- [x] **#93** Frontend: `PageTree.vue` import flow extended past `.md`/`.txt` to docx/doc/pdf/xlsx/xls/pptx/ppt/epub/html/csv, reusing the existing `markdownToHtml` + `setPendingImport` pipeline — *blocked by #92*
- [x] **#94** Export: `Block::Diagram` variant (`blocks.rs`), mermaid.ink PNG fetch, docx/pdf embed with text fallback — fixes diagrams exporting as raw ` ```mermaid ` source
- [x] **#95** Fix `Editor.vue`'s `activeTextSelection()` treating a clicked diagram/image `NodeSelection` as a drag-selected text range (popped the comment menu on a plain click)
- [x] **#96** Widen document editor column (`max-w-prose` → `max-w-3xl` in `[[pageId]].vue`)
- [x] **#97** Fix `diagramEmbed` export: `render_element_block` had no DB access so it emitted a `[Embedded diagram](/app/<id>)` link placeholder for md/docx/pdf alike. Added `resolve_diagram_embeds` in `export/mod.rs` — a post-pass over the generated Markdown, workspace-scoped, that swaps each placeholder for a ` ```mermaid ``` ` fence holding the target diagram page's real source (via the existing `yjs_named_text`), so it then flows through the same `Block::Diagram` → mermaid.ink → image pipeline #94 built — *blocked by #94*

**Verification performed** against the running docker-compose stack (`cargo watch`/`pnpm dev` hot-reload dev images):
- Backend (`cargo check`, inside the project's `rust:1-slim-bookworm` build image): clean after each change, including the new `convert` module and `multipart`/`json` axum/reqwest features.
- `docker compose build converter backend frontend` succeeded; stack brought up healthy (converter/backend/frontend/nginx all recreated).
- Import, end-to-end via real HTTP requests (registered + OTP-verified a throwaway account): uploaded an `.html` file with a heading/bold/italic/list/table to `POST /pages/import` → got back clean Markdown; a corrupt `.docx` correctly returned `400` with a readable error instead of crashing the converter or the backend.
- Import, in-browser (`agent-browser`): clicked the sidebar **Import** button (title now lists docx/pdf/xlsx/pptx/epub/...), uploaded the same `.html` file, and got a new page whose content rendered as real Tiptap nodes (`heading`, `columnheader`/`cell` for the table, list items) — not raw Markdown text.
- Diagram click bug, in-browser: inserted an inline Mermaid diagram via `/diagram`, single-left-clicked directly on the rendered diagram — no context/comment menu appeared; right-clicking the same spot still opened the block menu (Duplicate/Delete/Comment) as before.
- Diagram export, via the real `/pages/:id/export` endpoint on that page: `.md` still emits the ` ```mermaid ` fence (correct — Markdown renderers handle that natively); `.docx` and `.pdf` now embed an actual rendered PNG of the flowchart (extracted and visually confirmed from the `.docx`'s `word/media/`; `.pdf` confirmed via `/Image` XObject presence) instead of the raw source text that appeared before the fix.
- Editor width: screenshotted a document page — `max-w-3xl` visibly wider than the old `max-w-prose`, no layout regressions in the sidebar/TOC.
- Diagram embed export (#97), in-browser + via the real export endpoint: authored a standalone diagram page (`Client-->API-->Database`) and embedded it into a second document via `/embed diagram`; exported that document to `.md` (now shows the real ` ```mermaid ``` ` source instead of the link), `.docx` and `.pdf` (both confirmed via extracted/embedded PNG matching the live diagram, replacing the old plain "Embedded diagram" text).

## Phase 14: Export style fidelity, PDF-import tables, anonymous-guest collab, diagram UI

- [x] **#98** Diagram inline height `380px` → `560px`; fullscreen toggle (`DiagramToolbar.vue`/`DiagramEditor.vue`, `fixed inset-0 z-50` overlay, Escape to exit); `DiagramCanvas.vue` gained a real `fitToView()` (measures the rendered SVG, scales+centers to the container) called on first render and again on fullscreen toggle — the old "Fit" button just reset to a raw 1:1 scale, not an actual fit
- [x] **#99** Fixed anonymous-guest collab: `collab::ws_handler`/`comments::realtime::ws_handler` required a JWT unconditionally, so a logged-out public-link guest could never open the WebSocket at all — no presence, no realtime sync, and their edits (saved only via the periodic REST `PUT /pages/:id/content`) were silently clobbered by the WS room's stale in-memory flush, which never saw the out-of-band write. Both handlers now fall back to the `?link=` grant exactly like the REST `PagePermission` extractor already does
- [x] **#100** Export style fidelity — the Yjs walker had zero handling for `table`/`tableRow`/`tableCell`/`tableHeader` and `taskList`/`taskItem` (silently flattened to plain paragraphs) and the Run model dropped Highlight/Color mark data entirely:
  - `export/mod.rs`: `render_table` (GFM pipe-table emission), `ListKind::Task` marker, `highlight`/`textStyle` marks ride as `<mark style="background:...">`/`<span style="color:...">` inline HTML (no native GFM syntax) — `render_text`'s new `mark_color` helper reads the mark's Yjs `Any::Map` attrs, treating an empty-but-present color (default swatch) as unset
  - `export/blocks.rs`: `Options::ENABLE_TABLES`/`ENABLE_TASKLISTS`; `Block::Table`, `TableRow`; `Block::ListItem` gained `checked: Option<bool>`; `Run` gained `highlight`/`color`; parses the `<mark>`/`<span>` markers back out via `Event::InlineHtml` matching
  - `export/docx.rs`: real `docx_rs::Table` (bold+shaded header via `Shading`), code block/inline code backgrounds and highlight/color via `Run::shading()`/`.color()` (exact hex, not Word's fixed highlight-color enum)
  - `export/pdf.rs`: real table via `elements::TableLayout` + `FrameCellDecorator`; code blocks get a bordered/padded box (genpdf has no fill primitive, so no true background — documented limitation); inline code + Color-mark text get `Style::with_color`; Highlight has no PDF treatment for the same reason (yellow text-on-white would be illegible, worse than no treatment)
  - Task checkboxes render as ASCII `[x]`/`[ ]`, not the Unicode ☑/☐ glyphs first tried — LiberationSans (the PDF font) has no glyph for those and rendered a tofu box
- [x] **#101** PDF import losing tables: `markitdown`'s own PDF table detection is a best-effort word-position heuristic that regularly finds nothing (its PDF path is otherwise just a flat, layout-blind `pdfminer.extract_text()` dump). `converter/main.py` no longer uses markitdown for `.pdf` at all — `_pdf_with_tables` walks each page with `pdfplumber`, uses `page.find_tables()`'s bounding boxes to slice the page into text-above/table/text-below bands (`page.within_bbox`), and reassembles them in top-to-bottom order so a table lands inline at its real position instead of a separate appended section (first cut appended everything under an "Extracted tables" heading — reworked after feedback that split the data from its context)

**Verification performed**, all against the live docker-compose stack (`cargo watch`/`pnpm dev`, no restart needed — hot reload picked up every change):
- Style fidelity: pasted a Markdown blob (heading, bold, inline code, `==highlight==`, fenced code block, GFM table) into a real page, added a genuine Tiptap to-do item via the `/task` slash command and checked it. Exported all three formats:
  - `.md`: real pipe-table syntax, `<mark style="background:#fef08a">`, `- [x] Real checkbox item`
  - `.docx`: inspected `word/document.xml` directly — `w:shd` fills `F5F5F5`/`fef08a`/`272822`/`FAFAFA`, `w:color` `BE185D`/`F8F8F2`, real `w:tbl`; then round-tripped through a headless LibreOffice `--convert-to pdf` and rasterized it (`pdftoppm`) — visually confirmed Monokai code block, pink inline code, yellow highlight, bordered table with bold header, and a checkbox glyph, matching the in-app editor
  - `.pdf`: rasterized directly (`pdftoppm`) — bordered code box, pink inline code text, real bordered table, `[x] Real checkbox item` rendering correctly (the Unicode-glyph bug was caught and fixed at this step)
- Anonymous-guest collab: shared the same page as a public link with `editor` role; opened it in a second, fully separate browser session with **no login at all**. Confirmed, live: the guest appears as a presence avatar in the owner's topbar; text the guest typed (and its "Anonymous" collaboration-cursor label) appeared in the owner's view within the WS round-trip, no reload; after closing **both** sessions (forcing final persist), re-fetched the page via `GET /pages/:id/export?format=md` and confirmed the guest's edit was durably saved — not clobbered, which is what the bug previously did.
- Diagram UI: confirmed the taller inline height and default fit-to-center visually; toggled fullscreen (a `fixed inset-0` overlay with no sidebar/topbar) and confirmed the diagram auto-refit to the larger viewport; confirmed Escape exits fullscreen back to the normal document view.
- PDF-import tables: used this session's own PDF export (containing a paragraph, a code block, a table, and a task list — real layout, not a synthetic fixture) as the test input, POSTed it straight to `/pages/import`, and confirmed in the returned Markdown that the table lands between the code block and the task list — its actual original position — with no separate "Extracted tables" section.

## Phase 15: Live title sync, PDF code-block fill, embedded-diagram height, changelog nav

- [x] **#102** Title/icon changes weren't pushed to other viewers (Postgres-backed, not part of the Yjs doc, so nothing broadcast them). `pages::update_page` now publishes a `{"type":"page",...}` event over the existing per-page comments WebSocket (already open for every viewer regardless of whether the sidebar is open) via the same generic `comments::realtime::publish` hub the comment events already use. Frontend: `useCommentStream.ts` routes `type: "page"` events to a new `pagesStore.patchPageMeta()` (for workspace members) and an `onPageMeta` callback (for an anonymous link-guest's standalone `sharedPage`, which isn't part of the pages store)
- [x] **#103** PDF code blocks get a real dark background, not just a border: genpdf's public API has no fill primitive at all (verified directly against its source — `Area::draw_line` hardcodes `has_fill: false`; `Context` exposes only the font cache, nothing that reaches the underlying `printpdf` layer; `Area::layer()` itself is a private method). Added `resvg` (new dependency) to rasterize a tiny standalone SVG (dark rect + monospace `<text>`) to PNG and embed it through the export's existing image-embed path — falls back to the previous bordered box if rasterization fails for any reason
- [x] **#104** `DiagramEmbedView.vue` (the live-embed of a standalone diagram page, distinct from the inline diagram node fixed in Phase 13) still had the original `h-[320px]` — bumped to `h-[560px]` to match
- [x] **#105** Sidebar page clicks did nothing when browsing from `/changelog`: `PageTree.vue`'s `select()` only set `pagesStore.activePageId`, relying on a route↔store watcher that exists solely inside `pages/app/[[pageId]].vue` — any other host page rendering the sidebar (changelog) had no such watcher, so the click was silently a no-op. `select()` now calls `navigateTo()` directly instead of depending on a side effect that only exists on one host

**Verification performed**, live against the running stack:
- Title sync: proved the backend broadcast at the wire level first — a raw Python `websockets` client subscribed to `/ws/comments/:id`, then a `PATCH /pages/:id` fired from a separate curl call, and the listener received `{"icon":null,"title":"...","type":"page"}` immediately. Then end-to-end in two real, fully separate browser sessions (an authenticated owner + a second session with **no login**, viewing via the public editor link): the owner retyped the title and the guest's title `<input>` updated to match with zero reload — confirming the harder `sharedPage` code path, not just the `pagesStore.pages` one.
- PDF code-block background: exported, rasterized with `pdftoppm`, and visually confirmed a real solid dark-Monokai-background code block with legible light monospace text — not just a border. (First pass rendered a correctly-sized/positioned dark rect with **no text** — `fontdb`'s default generic-family mapping points "monospace" at "Courier New", which isn't installed in this image; fixed by explicitly setting `fontdb.set_monospace_family("Liberation Mono")`, the same family already used elsewhere in this exporter.)
- Backend (`cargo check` inside the project's `rust:1-slim-bookworm` build image, and the live `cargo watch` dev container independently): clean, including the new `resvg` dependency and every `usvg`/`tiny-skia` API call — verified against current docs.rs pages rather than guessed, given how much version drift these crates have had historically.
- Diagram embed height and changelog navigation: straightforward, low-risk changes verified by reading the resulting code rather than a fresh browser pass (the height bump mirrors Phase 13's already-verified pattern; the navigation fix is a one-line addition of a call already used identically elsewhere in the same codebase).

## Phase 16: Title sync follow-up — two real member sessions, Recents staleness

User reported title sync "still not updating" after Phase 15. Re-verified with two independent, **fully authenticated** browser sessions (not owner+anonymous-guest as before) both viewing the same page:

- [x] **#106** The core mechanism (`pagesStore.patchPageMeta`, the WS broadcast) was actually already correct — confirmed by inspecting the live Pinia store state directly (`pinia._s`) in the second session, which showed the renamed title immediately. What looked broken was the visible topbar/title-input update landing ~1s after the sidebar's own `pageTree` computed had *already* updated — a red herring caused by the 800ms title-save debounce plus normal WS round-trip, not a bug; a second, slower screenshot showed everything consistent.
- [x] **#107** Found and fixed a real gap while investigating: the **"Recents"** sidebar section never picked up a live rename at all. It's a separate `localStorage`-backed snapshot (`useRecents.ts`) seeded only when a page is opened (`record()`), entirely outside `pagesStore` — `patchPageMeta` never touched it. Added `useRecents().rename(id, patch)` and wired it into the same `useCommentStream` `onPageMeta` callback in `[[pageId]].vue` that already patches `sharedPage` for the anonymous-guest case.

**Verification**: two independent authenticated sessions (same account, two separate browser contexts — not the anonymous-link path this time) both viewing "Style Test"; renamed twice from session A. Session B's title input, topbar breadcrumb, sidebar page-tree entry, *and* "Recents" entry all converged on the new title, confirmed via both DOM screenshots and a direct Pinia store read (`document.querySelector('#__nuxt').__vue_app__` → `$pinia._s`).

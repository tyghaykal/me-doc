# Sharing UX + export enforcement + live presence/cursors

Companion to `tasks.md` — same work, broken into trackable tasks #55-#61.

## Context

Five related fixes to the page-sharing and realtime-collaboration experience:
1. A viewer can currently still open the Share dialog (its trigger button is gated on "logged in" only, not on role).
2. The Share dialog has no way to see who a page is already shared with.
3. There's no UI showing who's currently viewing/editing a document.
4. A viewer can currently still export a page — the button isn't hidden, and (more importantly) the backend doesn't enforce it either: `export_page` only checks workspace membership, not the page's resolved sharing role.
5. Document content already syncs live via the existing Yjs/WebSocket relay, but a collaborator's cursor/selection is invisible — Tiptap's `CollaborationCaret` extension is wired up and broadcasting correctly, but zero CSS renders what it emits, and the backend never sends a newly-joining client the *current* presence state of already-connected peers (only future changes).

Not in scope: live sync of title/icon/page-tree changes across clients — confirmed that gap exists too, but it's not what was asked (the request is specifically about document-body activity, which already syncs).

## Plan

### 1. Hide Share dialog from viewers
`AppTopbar.vue:65` "Private" badge → also gate on `!isViewer`, extend the read-only `v-else` badge to cover it.

### 2. Hide + enforce export restriction
Frontend: gate `ExportMenu` on `!isViewer`. Backend: switch `export_page` from `require_membership` (workspace-membership only) to the `PagePermission` extractor + `Role::Editor` check, matching `put_page_content`'s existing pattern.

### 3. List existing shares in the dialog
Backend: new `GET /pages/:id/permissions` (Editor-gated), one query joining `permissions`→`users` covering registered shares, pending invites, and public links. Frontend: `listShares`/`revokeShare` in `stores/pages.ts`, new "People with access" section in `ShareDialog.vue` reusing the existing `DELETE /permissions/:id`.

### 4. Visible realtime cursors/selections
Backend: send the room's current `Awareness::update()` snapshot to a newly-joining WS client right after `SyncStep1` (today it only gets future changes). Frontend: CSS for `.collaboration-carets__caret`/`.collaboration-carets__label` (currently zero rules — per-user color arrives via inline style already, CSS just needs shape/position/typography).

### 5. Presence list
Depends on #4. `Editor.vue` subscribes to `provider.awareness.on('change', ...)`, builds a `{clientId, name, color}` list, emits up through `[[pageId]].vue` into `AppTopbar.vue`, rendered as an overlapping avatar-circle stack near the edited-date/favorite cluster.

## Critical files
- `backend/src/export/mod.rs`, `backend/src/sharing/mod.rs`, `backend/src/collab/mod.rs`
- `frontend/app/components/AppTopbar.vue`, `Editor.vue`, `ShareDialog.vue`
- `frontend/app/stores/pages.ts`, `frontend/app/assets/css/main.css`

## Verification
Manual against the running docker-compose stack at `https://localhost` — no test suite in this repo. Share as viewer + confirm UI hidden and backend 403s; list/revoke shares as owner; two logged-in browser windows on the same page confirm live cursors (including a late joiner) and the presence stack updating with no refresh.

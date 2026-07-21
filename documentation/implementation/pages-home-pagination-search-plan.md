# Pages home, pagination, recents cleanup, and search

## Goals

1. **Home / `/app` (no page open)** — show all documents the user can access, paginated. First page shows **parent (root) pages only**; children load on expand.
2. **Sidebar** — same parent-first pagination so large workspaces don’t load the full tree at once.
3. **Recents** — never show or open deleted/archived pages.
4. **Search** — by page title, content, and creator/contributor name/email (not content-only FTS).

## Current state

| Area | Today |
|------|--------|
| `/app` empty state | Copy only: “Select a page from the sidebar.” |
| List pages | `GET /workspaces/:id/pages` returns **all** non-archived pages; frontend builds full tree client-side |
| Sidebar | `PageTree` renders full `pageTree` with no pagination |
| Recents | `localStorage` only (`useRecents`); no check against live pages / trash |
| Search | `GET /workspaces/:id/search?q=` — FTS on `page_content.search_vector` (from `plain_text` only); **title / creator not included** |
| Soft delete | `archived_at` on pages; trash list + restore exist |

## Approach

### A. Backend: paginated roots + children

**Endpoints (extend existing module, don’t invent a second store):**

1. `GET /workspaces/:workspace_id/pages?parent_id=&cursor=&limit=`
   - Default: `parent_id` absent / null → **root pages only** (`parent_page_id is null`).
   - `parent_id=<uuid>` → direct children of that page.
   - Always `archived_at is null`.
   - Pagination: `limit` (default 30, max 100), optional `cursor` = `order_index:id` of last item (stable sort: `order_index asc, id asc`).
   - Response:
     ```json
     {
       "items": [ /* Page + has_children */ ],
       "next_cursor": "12:uuid-or-null"
     }
     ```
   - Add `has_children: bool` (subquery / left join count) so the UI can show expand without loading kids.

2. Keep a **compat path** for small workspaces if needed: omit pagination params → same as today (all pages) **or** deprecate full dump once FE migrates. Prefer always-paginated roots for list; tree expand uses children endpoint.

3. **Shared + favorites** stay as-is for this phase (usually small); optional later pagination.

### B. Frontend: home document list

When `!activePage` and user has a workspace:

- New section in main canvas (or thin component `PageHomeList.vue`):
  - Grid/list of root pages (icon, title, updated_at).
  - “Load more” / infinite scroll using `next_cursor`.
  - Click → set `activePageId` / navigate `/app/:id`.
  - Expand chevron loads children via `parent_id` (same API).
- Empty workspace: keep create-page affordance.

### C. Frontend: sidebar pagination

- `pages` store:
  - Replace “fetch all then tree” with:
    - `rootPages` + `childrenByParentId` maps, or keep `pages` as a growing cache.
    - `fetchRootPages(workspaceId, { cursor })` / `fetchChildPages(parentId, { cursor })`.
  - `pageTree` computed from cached roots + loaded children only.
- `PageTree.vue`:
  - On expand (if not loaded): fetch children.
  - “Load more” under root list and under expanded parent when `next_cursor` present.
  - Collapse still client-only.

### D. Recents: drop deleted / inaccessible

- On `fetchPages` / after delete / on sidebar mount:
  - `useRecents().prune(validIds: Set<string>)` — remove entries whose id is not in active pages **and** not openable shared pages.
  - On select from Recents: if `GET /pages/:id` 404 or archived → remove from recents + toast, don’t open editor.
- On delete page: immediately remove id from recents (and favorites already handled server-side).
- Optional: store `workspace_id` on recent entry to avoid cross-workspace ghosts.

### E. Search: title + content + people

**Backend `search_pages` rewrite:**

Match if **any** of:

1. Title `ilike %q%` (or `to_tsvector` on title).
2. Existing `pc.search_vector @@ plainto_tsquery(...)` (content).
3. Creator: join `users` on `pages.created_by` — `email ilike` / `display_name ilike`.
4. Contributors: users who appear in:
   - `permissions` for that page (user principal), and/or
   - `comments` authors on that page  
   (cheap, no new table). Skip full Yjs author history for MVP.

Return same `Page` shape (+ optional `match_reason` later). Cap results (e.g. 50). Still exclude archived.

**Frontend `SearchPalette`:**

- Placeholder: “Search title, content, people…”
- Show subtitle under hit: title match / content / person if we add `match_reason`.
- Unchanged open/select UX (`Ctrl/Cmd+K`).

**Note:** content FTS only works if `plain_text` is maintained on write. Verify collab/PUT paths update `plain_text`; if not, add a follow-up task (out of scope unless broken).

## Critical files

| Layer | Files |
|-------|--------|
| Backend | `backend/src/pages/mod.rs` |
| Store | `frontend/app/stores/pages.ts` |
| UI | `PageTree.vue`, `AppSidebar.vue`, `[[pageId]].vue`, `SearchPalette.vue`, `SidebarRecents.vue` |
| Recents | `frontend/app/composables/useRecents.ts` |

## Tasks (implementation order)

1. **Backend paginated list** — roots/children + `has_children` + cursor  
2. **Store + PageTree pagination** — cache, expand-fetch, load more  
3. **Home document list** — empty `/app` main area  
4. **Recents prune** — delete path + prune on load + failed open  
5. **Search expand** — title + creator + contributor + palette copy  

## Verification

- Workspace with 50+ roots: sidebar loads first page only; Load more works; expand loads children only.
- `/app` with no selection shows paginated parents; open child after expand.
- Delete page: disappears from recents; click old recent doesn’t open editor.
- Search: find by title fragment, body word, creator email/display name, shared user email.
- Soft-deleted pages never appear in list/search/home.

## Out of scope

- Server-side full-text of collaborator cursors / Yjs CRDT authors  
- Infinite nested virtualization of deep trees  
- Cross-workspace global search  

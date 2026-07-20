# /app route: multi-workspace, user profile, page-create bug, font size

## Context

The `/app` route currently supports exactly one workspace per user (auto-created at registration) with no way to create more or invite anyone into it, and no way for a user to update their own name/avatar/password. Two smaller issues also need fixing: clicking "+ New page" on a brand-new (empty) workspace silently does nothing, and the UI text is a touch small. This plan covers all four, reusing this codebase's existing conventions throughout (Argon2 password hashing, the `permissions`/`workspace_members` tables, the `sharing.rs` invite pattern, the `pages.rs` presign-upload pattern, and the `ShareDialog.vue`/`VersionHistory.vue` Teleport-modal pattern) rather than introducing new ones.

Scope is deliberately kept tight for a small self-hosted app: no invite-by-email-for-nonexistent-user, no ownership transfer or workspace deletion, no avatar cropping, no email-change flow, no generic RBAC framework.

---

## C. Fix "+ New page" doing nothing (bug)

**Root cause** (confirmed by reading the code): `frontend/app/components/PageTree.vue:10-12`

```js
function workspaceId() {
  return props.nodes[0]?.workspace_id ?? pagesStore.pages[0]?.workspace_id
}
```

`addTopLevel()` guards on `if (ws) pagesStore.createPage(...)`. For a workspace with zero pages, both `props.nodes` and `pagesStore.pages` are empty arrays, so `workspaceId()` returns `undefined` and the click silently no-ops — no error, no request.

**Fix:**
- `frontend/app/pages/app/index.vue`: pass `authStore.workspace.id` (already available, used at lines 16/82) into `PageTree` as a new `workspace-id` prop: `<PageTree :nodes="pagesStore.pageTree" :workspace-id="authStore.workspace.id" />`.
- `frontend/app/components/PageTree.vue`: add `workspaceId: string` to `defineProps`, delete the `workspaceId()` inference helper, use `props.workspaceId` directly in `addTopLevel()` (and pass it down through recursive `<PageTree>` calls for children, or just keep using `parent.workspace_id` in `addChild` since that already works).
- `frontend/app/stores/pages.ts`: `createPage()` should set `activePageId` to the newly created page's id so the editor opens immediately instead of leaving the user on "Select a page from the sidebar."

---

## D. Font size (+~2px)

Tailwind v4, no config file — utilities are rem-based off the root `html` font-size (currently browser default 16px). Bumping the root scales every `text-*` utility proportionally in one place, hitting the dominant `text-sm` class (45 usages) along with everything else, without touching individual components.

**Fix:** `frontend/app/assets/css/main.css` — add `html { font-size: 18px; }` after the existing two lines.

---

## A. Multi-workspace support

### Backend — `backend/src/workspaces/mod.rs`

No migration needed — `workspaces` and `workspace_members` (migration `0001_init.sql`) already have everything: unique `slug`, `role check (role in ('owner','admin','member','guest'))`, composite PK `(workspace_id, user_id)` preventing double-membership.

Refactor the insert logic out of `create_default_workspace` into a shared helper, then add routes:

```rust
async fn insert_workspace(db: &PgPool, name: &str, owner_id: Uuid) -> Result<Workspace, AuthError> {
    let slug = format!("workspace-{}", &Uuid::new_v4().simple().to_string()[..8]);
    let (id,): (Uuid,) = sqlx::query_as(
        "insert into workspaces (name, slug, owner_id) values ($1, $2, $3) returning id",
    ).bind(name).bind(&slug).bind(owner_id).fetch_one(db).await?;
    sqlx::query("insert into workspace_members (workspace_id, user_id, role) values ($1, $2, 'owner')")
        .bind(id).bind(owner_id).execute(db).await?;
    Ok(Workspace { id, name: name.to_string(), slug })
}
```

`create_default_workspace` becomes a one-line call to `insert_workspace(db, "My Workspace", owner_id)`.

Router:

```rust
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_workspaces).post(create_workspace))
        .route("/:id", get(get_workspace))
        .route("/:id/members", get(list_members).post(add_member))
        .route("/:id/members/:user_id", delete(remove_member))
}
```

- `POST /workspaces` — `CreateWorkspaceRequest { name: String }`, reject empty/whitespace name via `AuthError::Validation`, calls `insert_workspace(&state.db, &name, user.user_id)`.
- `GET /workspaces/:id/members` — any existing member may view. `struct Member { user_id: Uuid, email: String, role: String }`, join `workspace_members ⋈ users`, order by `created_at asc`.
- `POST /workspaces/:id/members` — `AddMemberRequest { email: String, role: String }`. Add a `member_role(db, workspace_id, user_id) -> Option<String>` helper; require caller's role is `owner`/`admin` (`AuthError::Forbidden` otherwise). Validate `role` is `admin`/`member`/`guest` (no `owner` via this endpoint — no ownership transfer in scope). Look up target by email exactly like `sharing::share_with_user` (`sharing/mod.rs:58-63`) — `AuthError::Validation("no user with that email")` if the account doesn't exist (matches existing product behavior, no invite-for-nonexistent-user flow). Check for existing membership first and return `Validation("user is already a member")` rather than letting the composite-PK insert fail as a 500.
- `DELETE /workspaces/:id/members/:user_id` — allowed if caller is `owner`/`admin`, or if `user_id == caller.user_id` (self-leave). Reject removing `workspaces.owner_id` with `Validation("cannot remove the workspace owner")`.

No new `AuthError` variants needed. No nginx change — `/workspaces` is already proxied.

### Frontend

**New store `frontend/app/stores/workspaces.ts`** (kept separate from `auth.ts` — `authStore.workspace` stays "the active workspace," so its 4 existing read sites in `pages/app/index.vue` don't need to change *what* they read, only *when* it changes):

```ts
export interface WorkspaceMember { user_id: string; email: string; role: string }

export const useWorkspacesStore = defineStore('workspaces', () => {
  const api = useApi()
  const list = ref<Workspace[]>([])

  async function fetchAll() { list.value = await api<Workspace[]>('/workspaces'); return list.value }
  async function create(name: string) {
    const ws = await api<Workspace>('/workspaces', { method: 'POST', body: { name } })
    list.value.push(ws)
    return ws
  }
  const fetchMembers = (workspaceId: string) => api<WorkspaceMember[]>(`/workspaces/${workspaceId}/members`)
  const addMember = (workspaceId: string, email: string, role: string) =>
    api<WorkspaceMember>(`/workspaces/${workspaceId}/members`, { method: 'POST', body: { email, role } })
  const removeMember = (workspaceId: string, userId: string) =>
    api(`/workspaces/${workspaceId}/members/${userId}`, { method: 'DELETE' })
  function setActive(ws: Workspace) {
    useAuthStore().workspace = ws
    if (import.meta.client) localStorage.setItem('activeWorkspaceId', ws.id)
  }

  return { list, fetchAll, create, fetchMembers, addMember, removeMember, setActive }
})
```

**`frontend/app/pages/app/index.vue`:**
- On mount, after the existing `fetchPages` call: `workspacesStore.fetchAll()`, then restore the last-active workspace from `localStorage.getItem('activeWorkspaceId')` if present in the fetched list and different from `authStore.workspace` (login/refresh always seed the *first* workspace per `fetch_first_workspace` in `auth/mod.rs`, so this is how a non-default active workspace survives a reload).
- Add `watch(() => authStore.workspace?.id, (id, old) => { if (id && id !== old) { pagesStore.activePageId = null; pagesStore.fetchPages(id) } })` — single reaction point for any workspace switch, so the switcher/create-modal don't need to know about `pagesStore`.
- Replace the static `<h1>{{ authStore.workspace.name }}</h1>` with `<WorkspaceSwitcher />`.
- Add a settings gear button next to the theme toggle, opening `UserSettingsModal` (see Feature B).

**New `frontend/app/components/WorkspaceSwitcher.vue`** — anchored dropdown pattern copied from `ExportMenu.vue` (`relative` wrapper + `open` ref + `fixed inset-0` click-away layer + absolute panel), not a full Teleport modal. Shows active workspace name; panel lists `workspacesStore.list` (click → `setActive` → close); footer rows "+ New workspace" (opens `CreateWorkspaceModal`) and "Manage members" (opens `WorkspaceMembersModal` for the active workspace).

**New `frontend/app/components/CreateWorkspaceModal.vue`** — Teleport modal pattern from `ShareDialog.vue`. Single name input; submit → `create(name)` → `setActive(newWs)` → close.

**New `frontend/app/components/WorkspaceMembersModal.vue`** — same modal pattern, props `{ open: boolean; workspaceId: string }`. On open, `fetchMembers`. Lists members (email + role); remove buttons shown when the current user's own fetched role is `owner`/`admin` (self gets "Leave" instead). Invite form (email + role select: admin/member/guest) reuses `ShareDialog.vue`'s invite-form markup and `errText` error-handling pattern.

---

## B. User profile self-service (name, avatar, password)

### Migration — `backend/migrations/0008_user_profile.sql`

```sql
alter table users
    add column display_name text,
    add column avatar_key text;
```

`avatar_key` (not a full URL) matches the existing `attachments.s3_key` precedent — the frontend already builds display URLs client-side from `runtimeConfig.public.minioBase` + key (see `Editor.vue`'s `uploadImage`), so storing a full URL would duplicate that base and go stale across environments.

### Backend — new module `backend/src/users/mod.rs`

Mounted under the **existing** `/auth` prefix (`main.rs`: `.nest("/auth", auth::router().merge(users::router()))`) rather than a new top-level prefix, to avoid touching nginx's hardcoded proxy-path regex (`^/(health|auth|workspaces|pages|permissions|attachments|ws)(/|$)`). Routes: `GET/PATCH /auth/me`, `POST /auth/me/password`, `POST /auth/me/avatar/presign`.

```rust
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/me", get(get_me).patch(update_me))
        .route("/me/password", post(change_password))
        .route("/me/avatar/presign", post(presign_avatar))
}

#[derive(Serialize)]
pub struct MeResponse { id: Uuid, email: String, display_name: Option<String>, avatar_key: Option<String> }
```

- `get_me` — `select id, email, display_name, avatar_key from users where id = $1`.
- `update_me` — `UpdateMeRequest { display_name: Option<String> }`, `coalesce($2, display_name)` (same pattern as `pages::update_page`).
- `change_password` — `ChangePasswordRequest { current_password: String, new_password: String }`. Verify current via `password::verify_password` (else `AuthError::InvalidCredentials`), require `new_password.len() >= 8` (same check as `auth::register`), hash with `password::hash_password`, update. No OTP re-verification — login already gates via OTP.
- `presign_avatar` — mirrors `pages::presign_attachment` (`pages/mod.rs:331-361`) exactly, including its "write the key immediately, real object lands via the client's PUT" shortcut:
  ```rust
  let s3_key = format!("avatars/{}/{}-{}", user.user_id, Uuid::new_v4(), body.filename);
  let upload_url = crate::storage::presign_upload_url(&state.s3_presign, &state.config.s3_bucket, &s3_key, &body.content_type).await?;
  sqlx::query("update users set avatar_key = $2 where id = $1").bind(user.user_id).bind(&s3_key).execute(&state.db).await?;
  Ok(Json(json!({ "upload_url": upload_url, "s3_key": s3_key })))
  ```

Add `pub mod users;` to `backend/src/lib.rs`.

### Frontend

**New `frontend/app/components/UserSettingsModal.vue`** — Teleport modal pattern from `ShareDialog.vue`, three independent forms each with their own loading/error state (`errText` helper):
1. **Name** — loads current value via `GET /auth/me` when opened; submit → `PATCH /auth/me { display_name }`.
2. **Avatar** — `<input type="file" accept="image/*">` → `POST /auth/me/avatar/presign` → `fetch(upload_url, { method: 'PUT', body: file })` → preview via `${config.public.minioBase}/${s3_key}` (same `minioBase` already used in `Editor.vue`).
3. **Password** — `current_password` + `new_password` (+ client-side-only confirm field) → `POST /auth/me/password`.

Reachable via the gear-icon button added to `pages/app/index.vue`'s header (Feature A change), styled like the existing inline-SVG theme-toggle icon — no icon library.

---

## Files touched

**Backend:** `backend/migrations/0008_user_profile.sql` (new), `backend/src/workspaces/mod.rs`, `backend/src/users/mod.rs` (new), `backend/src/lib.rs`, `backend/src/main.rs`

**Frontend:** `frontend/app/stores/workspaces.ts` (new), `frontend/app/components/WorkspaceSwitcher.vue` (new), `frontend/app/components/CreateWorkspaceModal.vue` (new), `frontend/app/components/WorkspaceMembersModal.vue` (new), `frontend/app/components/UserSettingsModal.vue` (new), `frontend/app/components/PageTree.vue`, `frontend/app/stores/pages.ts`, `frontend/app/pages/app/index.vue`, `frontend/app/assets/css/main.css`

**Not touched:** `nginx/conf.d/default.conf`, `frontend/app/stores/auth.ts` (structure unchanged — `workspace` ref is simply reassigned by `setActive`)

## Verification

1. `docker compose up -d --build` (new migration runs automatically via `sqlx::migrate!` on backend startup).
2. Register/verify a user, land on `/app` with zero pages → click "+ New page" → page is created and opens immediately (Feature C).
3. Confirm UI text is visibly larger than before (Feature D).
4. Use the workspace switcher → "+ New workspace" → create one, confirm it becomes active and `pagesStore` refetches (empty page list for the new workspace). Switch back to the original workspace, confirm its pages reappear.
5. "Manage members" → invite a second, already-registered test account by email with role `member` → log in as that account → confirm it now sees the shared workspace in its switcher. Remove the member, confirm it disappears from their list on next fetch.
6. Open user settings → change display name, upload an avatar (confirm it renders via the presigned MinIO URL), change password → log out → log back in with the new password.

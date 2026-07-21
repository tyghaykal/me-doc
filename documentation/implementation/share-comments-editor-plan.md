# Share management, presence avatars, comments, and rich editor content

Companion to `tasks.md` — same work, broken into trackable tasks #62-#74.

## Context

Ten follow-up requests on top of the already-shipped Phase 11 (sharing UX + presence). They split into three groups:

**A. Share dialog polish (small, independent)**
1. Changing a share's role (viewer↔editor) currently requires deleting and re-adding the grant — no update-in-place endpoint exists.
2. Generating a public link doesn't auto-copy it to the clipboard.
3/4. The topbar presence stack shows only other users, only as colored initials, never the signed-in user themselves — despite the app already having a full avatar-upload system (`users.avatar_key`, MinIO presign, `UserSettingsModal.vue`'s `${minioBase}/${avatarKey}` pattern) that presence never taps into; `auth.ts`'s `User` type doesn't even carry `avatar_key`/`display_name`.

**B. Rich editor content**
6. Image resize — `@tiptap/extension-image@3.28.0` ships a native resizable node view, just never turned on (`resize: false` default).
7. Table support — no table extension installed; needs one added plus a slash-command entry and row/column controls.
8/10. Markdown paste/full support — StarterKit already gives typing-shortcut input rules for free; the real gap is pasted markdown text being treated as plain text (no MD→content parsing exists anywhere).
9. Import a `.txt`/`.md` file into a new page — no import affordance exists; reuses the paste-parsing logic from #8.

**C. Comments (new subsystem — nothing like it exists today)**
5. Select text, comment on it, optionally assign to someone by email, mark resolved — Google-Docs style. Confirmed by decision: comments are open to **viewers and editors**, and MVP scope is **single comment per range, no threaded replies**.

## Plan

### Group A — Share dialog polish

**#62 — Update a share's role in place.** Backend: `.patch()` on the existing `/permissions/:id` route in `sharing/mod.rs`, same Editor-only auth as `delete_permission`, `UPDATE permissions SET role = $1`. Frontend: `updateShareRole()` in `stores/pages.ts`; swap the read-only role text in `ShareDialog.vue`'s "People with access" rows for a `<select>` (reusing the existing invite/link role-select markup).

**#63 — Auto-copy public link after generating it.** `ShareDialog.vue`'s `generateLink()` calls the existing `copyLink()` right after `linkUrl.value` is set.

**#64 — Avatar-aware presence, self included.** Extend `auth.ts`'s `User` with `display_name`/`avatar_key`, hydrated via a `GET /me` call right after login/register so it's ready before any editor mounts. `Editor.vue`'s `currentUser` (feeds `CollaborationCaret`'s awareness state) gets `name`/`avatarUrl` from it; `updatePresence()` passes `avatarUrl` through. `AppTopbar.vue`'s avatar stack renders `<img>` when present else the existing initial-circle fallback, and gets a new "self" chip at the front of the stack sourced from `authStore.user` directly (closes both #3 and #4).

### Group B — Rich editor content

**#65 — Image resize.** `Image.configure({ resize: { enabled: true, ... } })` — native to the installed extension, config-only change.

**#66 — Table support via slash command.** Add `@tiptap/extension-table` (+`-row`/`-cell`/`-header`), wire into `Editor.vue`, add a "Table" slash-command entry, and a contextual `BubbleMenu` (shown only when `editor.isActive('table')`) with add/delete row/column buttons using Tiptap's built-in table chain commands. *Scope cut, stated up front:* a contextual toolbar, not full Notion-style hover/drag handles.

**#67 — Markdown paste support.** New small dependency `marked` + `frontend/app/utils/markdown.ts`'s `markdownToHtml()`. `Editor.vue`'s `handlePaste` runs plain-text-only clipboard content (no `text/html`) through it, then `insertContent()` — reuses ProseMirror's existing HTML deserialization rather than a custom MD parser. Typing shortcuts already work today via StarterKit, no change needed.

**#68 — Import a `.txt`/`.md` file.** *Blocked by #67.* New "Import file" affordance in `AppSidebar.vue`; `.md` goes through `markdownToHtml()`, `.txt` gets wrapped as paragraphs; creates a page and calls `editor.commands.setContent()` once its `Editor` mounts. Entirely client-side.

**#69 — Markdown round-trip verification.** *Blocked by #65-#68, verification only.* Confirms typing/paste/import/tables/images round-trip correctly both directions, including the pre-existing `export/mod.rs` Markdown export. Closes item #10 — no leftover gap once #65-#68 land.

### Group C — Comments (new subsystem)

**#70 — Backend: comments table + CRUD module.** New migration `0012_comments.sql` (`comments`: id, page_id, mark_id, author_id, assignee_id, body, resolved, created_at — anchor lives in the Yjs doc via the mark below, not as stored positions). New `backend/src/comments/mod.rs` mirroring `sharing/mod.rs`'s structure: `POST`/`GET /pages/:id/comments`, `PATCH /comments/:id/resolve`, `DELETE /comments/:id`, any resolved role (Viewer or Editor) may comment; resolve/delete needs Editor or the original author. Wire into `lib.rs`/`main.rs`. **Must also add `comments` to `nginx/conf.d/default.conf`'s route allowlist regex** — bit this project before in Phase 10 (`/me` 404s until added).

**#71 — Frontend: `comment` Tiptap mark.** New `comment-mark.ts`, `commentId` attribute, `excludes: ''` to allow overlapping comments. Rides on the same Y.Text custom-formatting-attribute mechanism already used for bold/italic — the anchor survives concurrent edits for free.

**#72 — Frontend: create a comment.** *Blocked by #70, #71.* "Comment" button in the existing selection `BubbleMenu`; small popover (body + assignee, filtered against the existing `GET /workspaces/:id/members`); applies the mark and POSTs to the backend with the same `commentId`.

**#73 — Frontend: comment sidebar.** *Blocked by #70, #71.* New `CommentSidebar.vue` + small `stores/comments.ts`; lists comments, resolve/delete, click-to-scroll to the anchor.

**#74 — Wire comments into the app shell.** *Blocked by #72, #73.* Toggle button in `AppTopbar.vue`, panel slot in `[[pageId]].vue`, muted highlight style for resolved comments.

## Critical files
- `backend/src/sharing/mod.rs`, `backend/src/comments/mod.rs` (new), `backend/migrations/0012_comments.sql` (new)
- `backend/src/main.rs`, `backend/src/lib.rs`, `nginx/conf.d/default.conf`
- `frontend/app/components/Editor.vue`, `AppTopbar.vue`, `ShareDialog.vue`, `slash-command.ts`
- `frontend/app/components/comment-mark.ts`, `CommentSidebar.vue` (new), `AppSidebar.vue`
- `frontend/app/stores/auth.ts`, `pages.ts`, `comments.ts` (new)
- `frontend/app/utils/markdown.ts` (new)
- `frontend/app/assets/css/main.css`

## Verification
Manual against the running docker-compose stack at `https://localhost`, real registered accounts + Mailpit OTP, `curl` for backend-only checks, in-browser for editor/UI:
- #62: change a grant's role, confirm `GET /pages/:id/permissions` reflects it with no duplicate row.
- #63: generate a link, confirm clipboard has it without clicking "Copy".
- #64: two sessions with avatars set — confirm real avatar images in the presence stack for both self and the other user.
- #65-#69: resize an image; insert/edit a table via slash command + contextual toolbar; type markdown shortcuts; paste markdown; import a `.md` file; confirm export round-trips all of it.
- #70-#74: comment as a viewer (confirm allowed); assign and resolve from another account; confirm a comment mark survives a concurrent edit from a second session (proves the anchor doesn't drift).

# Notion-style app shell redesign

Companion to `tasks.md` — same work, broken into trackable tasks #45-#54.

## Context

The authenticated app UI (`frontend/app/pages/app/[[pageId]].vue`) is currently a single 175-line file: a bare 256px sidebar containing only the page tree, a header of raw buttons doubling as a topbar, and a centered editor. This redesigns it to structurally follow Notion's layout (reference screenshot: `documentation/notion.png`) — a sidebar with a workspace switcher, an icon row, a Recents section, the existing page tree under a "Private" heading, a Shared section, and a pinned Trash footer; plus a real topbar (breadcrumb, visibility badge, edited date, share/copy-link, a "…" menu) over the content.

Two research passes confirmed the current implementation (components, stores, backend routes) and what data does/doesn't exist to back each Notion element. Notion-specific features with no equivalent here (Meetings/calendar, Agents, Library, My Tasks, Marketplace, AI chat bar) are intentionally not copied — only the structural pattern is being followed. Page icon/emoji and favorite/star both need new DB columns/tables; included as real, last-priority tasks rather than dropped.

## Approach

**Structural decision:** extract components, don't introduce a Nuxt layout — only one route uses this shell today, so `app/layouts/app.vue` would be pure speculation (YAGNI). `[[pageId]].vue` stays as the thin orchestrator (route↔store watchers, modal `ref`s, `onMounted`), everything visual moves into new components.

**Untouched:** `WorkspaceSwitcher.vue`, `Editor.vue` (Tiptap/Yjs), `ShareDialog.vue`, `VersionHistory.vue`, `CreateWorkspaceModal.vue`, `WorkspaceMembersModal.vue`, `UserSettingsModal.vue`, `ExportMenu.vue`, `PageTree.vue`'s drag/drop — only trigger buttons relocate into the new topbar/sidebar. Monochrome `neutral-*` Tailwind palette and `.dark`-class dark mode stay exactly as-is.

## Critical files
- `frontend/app/pages/app/[[pageId]].vue` — orchestrator, shrinks as sidebar/topbar extraction lands
- `frontend/app/components/PageTree.vue` — collapse/expand added, drag/drop preserved
- `frontend/app/stores/pages.ts` — gains `sharedPages`, `trash`, `searchPages`
- `backend/src/pages/mod.rs` — new `list_shared_pages` handler, pattern-matched to existing `list_pages`/`search_pages` (`PAGE_COLUMNS`/`PageRow`/`AuthenticatedUser`)
- `frontend/app/composables/useTheme.ts` — pattern reference for `useRecents.ts`

## Verification

No test suite exists in this repo — verification is manual against the running docker-compose dev stack at `https://localhost` (nginx TLS proxy). Per-task verification steps are in `tasks.md`.

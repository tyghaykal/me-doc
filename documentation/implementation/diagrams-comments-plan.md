# Collaborative Diagrams (Mermaid) + Real-time Comments

Companion to `tasks.md` — same work, broken into trackable tasks (Phases A–D below).

## Context

me-doc is a Notion-style collaborative doc app (Nuxt 4 / Vue 3 / TipTap / Yjs frontend,
Rust-Axum / Postgres / Redis / MinIO backend, bespoke `yrs` y-sync collab at `/ws/pages/:id`).
Two requests:

1. **Diagrams/charts** users can author (Mermaid-based, *all* diagram types), edit both as
   **Mermaid text with live preview** *and* via a **drag-and-drop canvas**, **co-edit in real
   time**, and **embed live into any document** — creatable **both** as standalone sidebar
   items **and** inline inside a document.
2. **Real-time comments** — comment create/reply/resolve/delete must push to every viewer over
   WebSocket so what a user sees is always current (today comments only refresh on sidebar-open /
   page-change / the local user's own action).

**Design intent (why this shape):** diagrams reuse the *entire* page subsystem instead of a
parallel `diagrams` table/store/route/collab-room. A diagram is **a page with `kind='diagram'`**,
which inherits CRUD, the `/ws/pages/:id` collab room, `page_content` persistence, sharing/
permissions, versions, sidebar, favorites, trash, and search — for the cost of one column. The
only genuinely new frontend surface is the diagram editor + embed node; the only new backend
surface is the comments WS fan-out. Frontend must be **hand-crafted, not AI-slop**: reuse the
app's existing Tailwind v4 tokens, neutral palette, dark-mode (`useTheme`), component idioms
(mirror `AppSidebar.vue` / `Editor.vue`), with real loading/empty/error states, keyboard support,
and files kept < 500 lines (per `CLAUDE.md`).

---

## Architecture

### Canonical model (one source of truth)
A diagram's content is its **Mermaid text**, stored in the Yjs doc as a `Y.Text` named `source`
(plus an optional `layout` `Y.Map` of per-node position/style overrides for the canvas). Mermaid
text is canonical because it natively expresses **every** diagram type and drives preview +
export. The visual canvas is a *projection* of `source`: it parses text → graph, applies `layout`
overrides, and writes edits back to `source`/`layout`. Text view and visual view therefore never
diverge — there is one truth.

Because content rides the existing Yjs doc, **real-time collaboration on diagrams is free** (both
standalone diagram-pages via `/ws/pages/:id` and inline blocks via their host page's doc). No
backend collab changes.

### Type coverage without a 15-way mess — adapter registry
`frontend/app/utils/diagram/adapters/` — a small registry keyed by Mermaid diagram type. Each
adapter implements:
```ts
interface DiagramAdapter {
  type: string                                   // 'flowchart' | 'stateDiagram' | ...
  parse(source: string): { nodes: FlowNode[]; edges: FlowEdge[] }
  generate(model, prevSource): string            // model -> mermaid text (round-trip)
  canvasComponent: Component                      // Vue Flow-based editor for this family
}
```
- **Ship the `flowchart`/`graph` adapter** (the dominant diagramming case) with **full round-trip**
  drag-drop via **Vue Flow** (`@vue-flow/core`).
- Graph-like families (`stateDiagram`, `erDiagram`, `classDiagram`, `mindmap`) follow the *same
  adapter shape* — each is a bounded, repeatable add, not a rewrite.
- **Any type without an adapter automatically falls back to text + live preview** (still fully
  supported: create, collaborate, embed, comment, export, render). So **all diagram types work on
  day one**; drag-drop lights up per adapter. This is the honest "covers all types" answer — the
  registry seam is justified because there are genuinely N implementations (one per type), so it is
  not a speculative abstraction.

> Temporal/statistical types (sequence, gantt, pie, journey, timeline, gitGraph) are **text + live
> preview** for structure; a drag-drop canvas for them is a later adapter, not part of the first
> cut. "Drag a pie slice" is not a real operation — for those types the canvas pane is a
> zoom/pan/fit live preview.

### Editor UI (`DiagramEditor.vue`, split into small components)
Split view, matching the app's design language:
- **Code pane** (`DiagramCodePane.vue`): collaborative Mermaid text (Yjs `source`), monospace,
  live parse-error underline from `mermaid.parse()`, collaborator carets (reuse the
  `CollaborationCaret` presence pattern from `Editor.vue`).
- **Canvas pane** (`DiagramCanvas.vue`): always-on live preview; for adapter-backed types an
  interactive Vue Flow overlay (drag-reposition, add/connect/delete/relabel → regenerate text);
  otherwise the rendered SVG with zoom/pan/fit.
- **Toolbar** (`DiagramToolbar.vue`): view toggle (Text / Split / Visual), diagram-type picker
  (inserts a starter template), zoom/fit, export, live collaborator avatars (reuse presence).
- Mermaid theme synced to app light/dark via `useTheme`.

### Two homes, one editor
- **Standalone diagram page:** `pages.kind='diagram'`. Created from the sidebar ("New diagram").
  The app shell `pages/app/[[pageId]].vue` renders `<DiagramEditor>` when
  `activePage.kind==='diagram'`, else the existing `<Editor>`. Same `/app/:id` route — no new route.
- **Inline diagram block:** a TipTap node `diagram` (mirror `container-node.ts`, but an
  `atom` block whose `source` is a node attribute; NodeView = a compact `DiagramEditor`). Inserted
  via a `/diagram` slash-command entry. Source rides the host page's Yjs doc → collaborative free.

### Live-linked embedding
TipTap atom node `diagramEmbed` storing `diagramId`. NodeView (`DiagramEmbedView.vue`) renders that
diagram's **current** source read-only via mermaid, with an "open" affordance. **Live:** on mount it
fetches current content; while open it subscribes **read-only** to the diagram's `/ws/pages/:id`
room (reuse `WebsocketProvider`) and re-renders on change. Inserted via a `/embed diagram` slash
entry that opens a picker listing the workspace's `kind='diagram'` pages (reuse the
`onInsertImage`-style callback option in `slash-command.ts`).

### Real-time comments (new backend WS, in-process fan-out)
Mirror the collab `DocRoom` broadcast pattern, minus the CRDT — comments are DB rows, so just fan
out JSON:
- `AppState` gains `comments: CommentHub` = `Arc<DashMap<Uuid, broadcast::Sender<String>>>`
  (page_id → subscribers).
- Route `GET /ws/comments/:pageId` (JWT verified on upgrade + `sharing::resolve_role` gate; viewers
  allowed, matching comment read access). Read-only: server → client events only.
- Each mutating handler in `comments/mod.rs` (`create_comment`, `add_reply`, `resolve_comment`,
  `delete_comment`) publishes a typed event (`{type:'created'|'updated'|'deleted', comment|id}`)
  **after** the DB write.
- Frontend `useCommentStream(pageId)` composable opens the socket while a page is open and applies
  events to `commentsStore.comments`, so `CommentSidebar.vue` (which already renders from the store
  and toggles `comment-resolved` highlights) is always current — including resolve state — even
  when the sidebar is closed.
- Ceiling: single-instance in-process fan-out (matches today's single `backend` service). A
  `ponytail:` comment marks the upgrade path to **Redis pub/sub** when horizontally scaled (Redis
  client already in `AppState`).

### Backend export + search
`export/mod.rs` `render_element_block` currently drops unknown nodes' non-text content. Add:
- `"diagram"` → emit a ```` ```mermaid ```` fence from the node's `source` attr (`attr_str`).
- `"diagramEmbed"` → emit the referenced diagram's source as a ```` ```mermaid ```` fence (fetch by
  id) or a link placeholder.
`put_page_content`: for `kind='diagram'` pages, store the Mermaid `source` as `plain_text` (so
diagram source is searchable) instead of running the doc-body markdown walker.

---

## Files

**Migrations (backend)**
- `backend/migrations/0014_page_kind.sql` — `alter table pages add column kind text not null default 'document';`
  (Sharing/permissions/collab are already generic over page id — no other schema change. Comments
  realtime needs no schema.)

**Backend — Rust**
- `backend/src/pages/mod.rs` — `Page` gains `kind`; `create_page` accepts optional `kind`; list/get
  return it. New-diagram creation reuses `create_page`.
- `backend/src/comments/mod.rs` — publish events after each mutation; helper to serialize a comment.
- `backend/src/comments/realtime.rs` (new) — `CommentHub`, `/ws/comments/:pageId` handler, publish
  helper. Mirror the auth/upgrade shape of `collab/mod.rs`.
- `backend/src/lib.rs` / `main.rs` — add `comments` hub to `AppState`; merge the comments WS route.
- `backend/src/export/mod.rs` — `diagram` + `diagramEmbed` cases.

**Frontend — Vue/TS** (all under `frontend/app/`)
- `components/DiagramEditor.vue`, `DiagramCodePane.vue`, `DiagramCanvas.vue`, `DiagramToolbar.vue`
  (new; each < 500 lines).
- `components/diagram-node.ts` (inline `diagram` TipTap node), `components/diagram-embed.ts`
  (`diagramEmbed` node) + `components/DiagramEmbedView.vue` (NodeView).
- `utils/diagram/` — `mermaid.ts` (render/parse/theme-sync helpers), `adapters/index.ts` (registry),
  `adapters/flowchart.ts` (first adapter).
- `composables/useCollab.ts` (new) — extract the `WebsocketProvider` + presence setup currently
  inline in `Editor.vue` so `Editor.vue`, `DiagramEditor.vue`, and the embed subscription share one
  implementation. `composables/useCommentStream.ts` (new).
- Wire-ups: `components/Editor.vue` (register `diagram` + `diagramEmbed` in the `extensions` array;
  add slash callbacks), `components/slash-command.ts` (`/diagram`, `/embed diagram` entries +
  picker callbacks), `components/SlashCommandList.vue` (picker if needed),
  `pages/app/[[pageId]].vue` (render `DiagramEditor` when `kind==='diagram'`),
  `components/PageTree.vue` / sidebar ("New diagram" action, diagram icon), `stores/pages.ts`
  (`createPage` passes `kind`; `Page` type gains `kind`), `pages/app/[[pageId]].vue` +
  `CommentSidebar.vue` (mount `useCommentStream`).
- `frontend/package.json` — add `mermaid`, `@vue-flow/core` (+ `@vue-flow/background`,
  `@vue-flow/controls`). All client-only; `/app/**` is already `ssr:false`.

**Reused verbatim (no change):** sharing (`sharing/mod.rs`, `ShareDialog.vue`), collab room
(`collab/mod.rs`), versions, favorites/trash, `useApi`/`useApiBase`, presence/awareness plumbing.

---

## Phased task breakdown

**Phase A — Real-time comments (self-contained, ships value immediately)**
- A1. `CommentHub` in `AppState` + `comments/realtime.rs` WS route (auth + role gate).
- A2. Publish events from the four comment mutations — *blocked by A1*.
- A3. `useCommentStream` composable + apply events to store; mount in app shell — *blocked by A2*.
- A4. Verify two browsers: comment/reply/resolve/delete reflect live both ways.

**Phase B — Diagram core (text + preview + collab + standalone/inline)**
- B1. Migration `0014_page_kind` + backend `kind` in create/list/get.
- B2. Add `mermaid` dep + `utils/diagram/mermaid.ts` (render + parse + theme sync).
- B3. `useCollab` composable extracted from `Editor.vue` (refactor, no behavior change) — *blocked by B2*.
- B4. `DiagramEditor` + `DiagramCodePane` (collaborative text) + `DiagramCanvas` (live preview) +
  `DiagramToolbar` (type picker/templates/export) — *blocked by B1, B2, B3*.
- B5. App shell renders `DiagramEditor` for `kind==='diagram'`; sidebar "New diagram" — *blocked by B4*.
- B6. Inline `diagram` TipTap node + NodeView + `/diagram` slash entry — *blocked by B4*.
- B7. Export: `diagram` fence in `export/mod.rs`; diagram-page search `plain_text` — *blocked by B1, B6*.

**Phase C — Visual drag-drop (adapter + Vue Flow)**
- C1. Add `@vue-flow/*` deps; adapter registry (`utils/diagram/adapters/index.ts`) — *blocked by B4*.
- C2. `flowchart` adapter: `parse` + `generate` (round-trip) — *blocked by C1*.
- C3. Vue Flow canvas in `DiagramCanvas.vue`: drag-reposition (`layout` Y.Map), add/connect/delete/
  relabel → regenerate `source`; non-adapter types fall back to preview — *blocked by C2*.
- C4. Round-trip self-check: text→model→text is stable for the flowchart adapter.

**Phase D — Live embedding**
- D1. `diagramEmbed` node + `DiagramEmbedView.vue` (read-only render) — *blocked by B4*.
- D2. Live: subscribe read-only to the diagram's `/ws/pages/:id` via `useCollab` — *blocked by D1, B3*.
- D3. `/embed diagram` slash entry + workspace-diagram picker — *blocked by D1*.
- D4. Export: `diagramEmbed` → mermaid fence / placeholder — *blocked by D1, B7*.

---

## Verification

- **Build/tests:** `cd backend && cargo build && cargo test`; `cd frontend && npm run build`.
  Stack up via `docker-compose up`.
- **Comments realtime (A):** open the same page in two browsers; add/reply/resolve/delete in one →
  appears in the other within a moment without manual refresh; resolved highlight toggles live.
- **Diagram collab (B):** create a diagram from the sidebar in two browsers; type Mermaid in one →
  the other's text + preview update live; refresh → content persisted (`page_content.yjs_state`).
- **Inline diagram (B6):** `/diagram` in a doc → block renders; edits sync to a second viewer.
- **Visual round-trip (C):** on a flowchart, drag a node and add an edge on the canvas → Mermaid
  text updates; edit the text → canvas reflects it; no divergence.
- **Embed live (D):** `/embed diagram` in doc B referencing diagram X; edit X in another tab → the
  embed in B re-renders live; export B (DOCX/PDF/MD) contains the diagram as a mermaid fence.
- **All-types smoke:** switch the type picker through sequence/gantt/pie/class/ER/state/mindmap/
  timeline → each renders in preview and exports; adapter-less types cleanly show preview-only
  (no broken canvas).

## Scope notes (deliberate first-cut limits)
- Drag-drop canvas ships for **flowchart** first; other graph-like types follow the same adapter
  shape; temporal/statistical types are text+preview (still fully supported). — *add adapters as needed.*
- Comments WS is **single-instance in-process** fan-out. — *swap to Redis pub/sub when the backend
  is horizontally scaled.*

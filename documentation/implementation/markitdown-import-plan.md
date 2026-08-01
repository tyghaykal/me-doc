# MarkItDown import + diagram export/editor fixes

## Goals

1. **Document import as a service** — user picks docx/doc/xls/xlsx/pdf/epub/pptx/etc. from "Import"; `.txt`/`.md` still import client-side (already built); everything else is converted to Markdown by a MarkItDown-backed service and fed through the same pipeline.
2. **Fix diagram export** — Mermaid diagrams currently export as raw ` ```mermaid ` source text in DOCX/PDF instead of a rendered image.
3. **Fix diagram click → comment popup bug** — clicking (not right-clicking) a diagram opens the same context menu as a right-click.
4. **Widen the editor column** — document pages render at `max-w-prose`, noticeably narrower than the empty-state/diagram columns.

## Existing groundwork (found while reading the code, not building from scratch)

- `PageTree.vue` already has a working **Import** button for `.txt`/`.md`: reads the file client-side, converts `.md` via `markdownToHtml()` (`frontend/app/utils/markdown.ts`, a `marked` wrapper already used for paste-as-markdown), creates a page, and stashes the HTML via `pagesStore.setPendingImport(pageId, html)`.
- `Editor.vue`'s `onMounted` already drains that pending import (`pagesStore.takePendingImport`) and calls `editor.value?.commands.setContent(importedHtml)` once the fresh page's editor/Yjs doc exist.
- Net effect: **no new client-side content pipeline is needed.** The only gap is turning docx/pdf/xlsx/... bytes into Markdown text — everything downstream (Markdown → HTML → Yjs doc, via Tiptap's Collaboration extension) already works.

## A. Import service

### Architecture
A file goes browser → backend (multipart, auth-checked) → `converter` microservice (Python, wraps `microsoft/markitdown`) → Markdown text → back to backend → browser. The browser then runs the *same* `markdownToHtml` + `setPendingImport` + `createPage` path `.md` imports already use.

Rust has no maintained MarkItDown port, so the conversion itself has to live in Python — matches the user's framing ("pass to this service") and mirrors this repo's existing pattern of small, single-purpose services in `docker-compose.yml` (postgres, redis, minio, mailpit all internal-only, no code of ours).

### New service: `converter/`
- `converter/main.py` — FastAPI, two routes: `GET /health`, `POST /convert` (multipart `file` field). Runs `MarkItDown().convert_stream(...)`, returns `{"markdown": "..."}`. 4xx on unsupported/corrupt input, mapped from MarkItDown's exceptions.
- `converter/requirements.txt` — `markitdown[all]`, `fastapi`, `uvicorn[standard]`, `python-multipart`.
- `converter/Dockerfile` — `python:3.12-slim`, install requirements, `uvicorn main:app --host 0.0.0.0 --port 8000`.
- `docker-compose.yml` — new `converter` service, **no published port** (internal-network only, same as postgres/redis/minio), healthcheck via `/health`; `backend` gets `CONVERTER_URL` env + `depends_on: converter`.

### Backend
- `Cargo.toml` — add `multipart` feature to `axum` (only new dependency surface; everything else — `reqwest`, `base64` — already present).
- `config.rs` — `converter_url: String` (env `CONVERTER_URL`, default `http://converter:8000`).
- New `backend/src/convert.rs`:
  - `router()` → `POST /pages/import` (path deliberately under the existing `/pages` prefix so nginx's location regex — `^/(...|pages|...)(/|$)` — proxies it with zero nginx changes).
  - Handler: `AuthenticatedUser` (any logged-in user; stateless proxy, no workspace/page row touched), reads one multipart field, caps bytes (20 MiB, matching nginx's `client_max_body_size 20m`), re-poses the bytes as multipart to `{converter_url}/convert`, returns `{"markdown": ...}`. Converter-unreachable / conversion failure → `AuthError::Validation` with a user-facing message (no separate `AppState` field needed — build the reqwest client inline, same as `export/blocks.rs::fetch_images` already does).
- `lib.rs` — `pub mod convert;`; `main.rs` — `.merge(convert::router())`.

### Frontend
- `PageTree.vue`:
  - Widen `accept` on the hidden file input to MarkItDown's common inputs: `.md,.txt,.docx,.doc,.pdf,.xlsx,.xls,.pptx,.ppt,.epub,.html,.csv`.
  - `onImportFile`: keep the existing local branch for `.md`/`.txt`; for anything else, `POST` a `FormData` to `/pages/import`, take back `{markdown}`, run it through the same `markdownToHtml()` call the `.md` branch already uses. Add a small `importing`/`importError` ref pair (mirrors `ExportMenu.vue`'s `downloading`/`error` pattern) so the button shows "Importing…" and failures surface inline instead of silently doing nothing.
  - Button title/label updates from "Import a .txt or .md file" to something reflecting the wider format list.

### Out of scope
- No OCR/plugin config for MarkItDown beyond its default `[all]` extras.
- No virus/malware scanning of uploaded files (matches existing attachment upload, which also has none).
- No progress streaming for large files — single request/response, capped at 20 MiB.

## B. Diagram export fix

**Root cause:** `blocks::parse_markdown` collapses every fenced code block (including ` ```mermaid `) into `Block::Code(String)`, discarding the language. `docx.rs`/`pdf.rs` render `Block::Code` as literal monospace text — so a diagram exports as raw Mermaid source, not a picture.

- `blocks.rs`: track the fence's language (`Tag::CodeBlock(CodeBlockKind::Fenced(lang))`); when `lang == "mermaid"`, push a new `Block::Diagram(String)` instead of `Block::Code`. Add `collect_diagram_sources(&[Block]) -> Vec<String>` (unique), mirroring `collect_image_urls`.
- `export/mod.rs`: after parsing blocks for docx/pdf, fetch a rendered PNG per unique diagram source from `mermaid.ink` (`GET https://mermaid.ink/img/{url_safe_base64(source)}`) — same fetch shape as `blocks::fetch_images` (10s timeout, size cap, best-effort/omit-on-failure). Build a second `HashMap<String, Vec<u8>>` and pass it alongside the existing `images` map into `blocks_to_docx`/`blocks_to_pdf`.
- `docx.rs` / `pdf.rs`: new `Block::Diagram(source)` arm — if a rendered PNG exists, embed it exactly like `Block::Image` (reusing `embed_image`/`make_pdf_image`); otherwise fall back to the current mono-text rendering (offline/mermaid.ink-unreachable degrades to readable source instead of a missing diagram).

`mermaid.ink` is a public third-party renderer — the lazy option given a Rust-only backend with no headless browser. Noted as a `ponytail:` comment (external dependency, self-hosted/air-gapped deployments fall back to text) rather than standing up a Node/headless-Chrome render service for this alone.

## C. Diagram click → comment-popup bug

**Root cause:** `DiagramNode` (and Tiptap's built-in `Image`) are `atom: true, selectable: true` — a single click on one produces a ProseMirror `NodeSelection`, not a `TextSelection`. `Editor.vue`'s `activeTextSelection()` only checks `selection.empty`, which is `false` for a `NodeSelection` too, so `onEditorMouseUp` treats "clicked to select a diagram" the same as "dragged to select text" and pops the same menu a right-click/drag-select would (which includes "Add comment").

- `Editor.vue`: import `NodeSelection` from `@tiptap/pm/state`; `activeTextSelection()` also returns `undefined` when `selection instanceof NodeSelection`. One shared helper, so this also fixes the same latent bug for clicking images, not just diagrams — root-cause fix, not a diagram-specific patch.

## D. Editor width

`frontend/app/pages/app/[[pageId]].vue`: document pages use `max-w-prose` (narrower than both the empty-state's `max-w-3xl` and the diagram view's `max-w-5xl`). Bump the document branch to `max-w-3xl` — one class, matches the empty-state width already used elsewhere on the same page.

## Files touched

| Area | Files |
|------|-------|
| Converter service | `converter/main.py`, `converter/requirements.txt`, `converter/Dockerfile` (new) |
| Compose/deploy | `docker-compose.yml` |
| Backend import route | `backend/src/convert.rs` (new), `backend/src/lib.rs`, `backend/src/main.rs`, `backend/src/config.rs`, `backend/Cargo.toml` |
| Frontend import UI | `frontend/app/components/PageTree.vue` |
| Diagram export | `backend/src/export/blocks.rs`, `backend/src/export/mod.rs`, `backend/src/export/docx.rs`, `backend/src/export/pdf.rs` |
| Diagram click bug | `frontend/app/components/Editor.vue` |
| Editor width | `frontend/app/pages/app/[[pageId]].vue` |

## Task breakdown

1. **converter-service** — FastAPI + MarkItDown wrapper, Dockerfile, compose wiring
2. **backend-import-route** — multipart endpoint proxying to the converter — *blocked by #1*
3. **frontend-import-ui** — extend `PageTree.vue`'s import flow to non-md/txt files — *blocked by #2*
4. **export-diagram-fix** — `Block::Diagram`, mermaid.ink fetch, docx/pdf embed
5. **editor-node-selection-fix** — `activeTextSelection()` excludes `NodeSelection`
6. **editor-width** — `max-w-prose` → `max-w-3xl`

Tasks 4–6 are independent of 1–3 and of each other.

## Verification

- Import a `.docx`/`.pdf`/`.xlsx` with headings/lists/tables → new page created, content readable and editable, saves/syncs normally.
- Import a `.txt`/`.md` still works unchanged (no network round-trip).
- Unsupported/corrupt file → inline error, no half-created page left behind... *(page is already created before conversion in the `.md` path; for the new path, create the page only after a successful conversion so a failed import doesn't leave an empty page.)*
- Export a page containing a Mermaid diagram to DOCX and PDF → diagram renders as an image, not raw source text; export still works when mermaid.ink is unreachable (falls back to text, doesn't error the whole export).
- Click (not drag, not right-click) a diagram node → no context/comment menu appears; right-click still opens it.
- Editor column is visibly wider on a document page; diagram/empty-state widths unchanged.

# Export fidelity + comment replies plan

## Goals

1. **DOCX export** — render markdown styles (headings, bold/italic/code/strike, lists, quotes, code blocks) and embed images. Fix cases where Word shows blank/unstyled content.
2. **PDF export** — same style fidelity gaps; embed images; improve code-block presentation.
3. **Comments** — per-comment replies; commented text stays highlighted; click highlight opens that comment.

## Root causes (current code)

### Export
- Pipeline: Yjs → Markdown (`export/mod.rs`) → `blocks::parse_markdown` → `docx.rs` / `pdf.rs`.
- Images intentionally degraded to `[image: alt]` in `blocks.rs` (no fetch/embed).
- DOCX: no `Pic` usage; code blocks are plain mono lines with no shading; run fonts only set `ascii` (Word can drop hiAnsi/cs).
- PDF: genpdf built without `images` feature; no mono font; code blocks are just smaller body font; no image blocks.
- Image URLs in MD are typically MinIO public URLs (`minioBase/s3_key`) — backend can HTTP-fetch them at export time.

### Comments
- Table has unique `mark_id`; one row per anchor; no `parent_id` → no replies.
- Sidebar lists flat comments; no reply UI.
- Highlight CSS exists (`.comment-highlight`) but no click handler to open the sidebar on the mark.

## Approach

### A. Shared export model + images
1. Add `reqwest` (rustls) for image download at export time.
2. Extend `Block` with `Image { alt, bytes, width, height }` (or raw bytes + format).
3. In `parse_markdown`, on `Tag::Image { dest_url, .. }` collect URL; after parse (or during export), fetch bytes. Prefer fetch in `export_page` / a small `images` helper so DOCX/PDF share one cache.
4. Simpler path: `collect_image_urls` from markdown → `fetch_images(urls) -> HashMap<Url, Bytes>` → pass map into `markdown_to_docx` / `markdown_to_pdf`. Parser emits `Block::Image { alt, url }`; renderers look up bytes.

### B. DOCX fidelity
1. `build_run`: set `RunFonts` with ascii + hi_ansi + cs for body and mono; apply bold/italic/strike/code.
2. Code blocks: mono + light gray highlight/shading if API allows; keep line-per-paragraph.
3. Images: `Pic::new(&bytes).size(w, h)` on a run inside a paragraph (cap width ~500px / EMUs).
4. Headings: keep size+bold; ensure non-empty runs always emit at least one run.
5. Empty paragraphs: skip empty run lists that produce blank-only docs only when truly empty content.

### C. PDF fidelity
1. Enable `genpdf` feature `images`.
2. Load LiberationSans + LiberationMono (or DejaVu) from same font dirs.
3. Code blocks: mono font + slightly smaller size; optional framed element.
4. Inline code: mono font via Style if API allows font override — genpdf Style is bold/italic/size; for mono use separate font family on document or styled element. Practical approach: body family for text; for code paragraphs use mono family if `Document` supports switching — genpdf Document is single family at construction. **Workaround:** keep one family but prefix code with distinct size; OR build Document with LiberationSans and for code use a second approach. Checking genpdf: fonts are per-document family. Load LiberationSans as main; code stays smaller. Mono is best-effort if we can pass FontFamily per element — genpdf Style doesn't switch family easily. Ship: smaller size + indent for code; images via `elements::Image::from_dynamic_image` / `from_reader`.
5. Images: decode with `image` crate, strip alpha if needed (genpdf rejects alpha), scale to max width ~160mm.

### D. Comment replies + click-to-open
1. Migration `0013_comment_replies.sql`:
   - `parent_id uuid references comments(id) on delete cascade`
   - Drop unique `idx_comments_mark_id`
   - Partial unique: `unique (mark_id) where parent_id is null` (one root per mark)
   - Index `(page_id, parent_id)`, `(parent_id)`
2. Backend:
   - Comment DTO gains `parent_id: Option<Uuid>`
   - Create accepts optional `parent_id`; if set, inherit `page_id` from parent, set `mark_id` to parent's mark_id (or allow null on replies — use parent's mark_id for scroll targeting)
   - List returns flat list ordered by created_at; frontend groups threads
   - Delete cascade via FK for replies when root deleted
3. Frontend store:
   - `addReply(parentId, body)`
   - Group helpers: roots + children by parent_id
4. `CommentSidebar.vue`:
   - Thread UI under each root
   - Reply input per thread
   - `focusedMarkId` prop — scroll/highlight that card when opened from editor
5. Editor click:
   - On click of `[data-comment-id]`, emit `open-comment` with markId
   - Page opens sidebar and passes focused mark
   - Ensure marks stay styled (already `.comment-highlight`)

## Files

| Area | Files |
|------|--------|
| Export model | `backend/src/export/blocks.rs` |
| DOCX | `backend/src/export/docx.rs` |
| PDF | `backend/src/export/pdf.rs` |
| Export entry + fetch | `backend/src/export/mod.rs`, `backend/Cargo.toml` |
| Comments schema | `backend/migrations/0013_comment_replies.sql` |
| Comments API | `backend/src/comments/mod.rs` |
| Comments store | `frontend/app/stores/comments.ts` |
| Sidebar | `frontend/app/components/CommentSidebar.vue` |
| Editor click | `frontend/app/components/Editor.vue`, `[[pageId]].vue` |
| Mark CSS | `frontend/app/assets/css/main.css` (if needed) |

## Task breakdown

1. **export-model-images** — Block::Image, parse image URLs, shared HTTP fetch helper, wire into export_page
2. **export-docx** — style fixes + Pic embed from fetched bytes
3. **export-pdf** — images feature, style fixes, image embed
4. **comments-backend** — migration + parent_id create/list
5. **comments-frontend** — store replies, threaded sidebar, click mark → open

## Agent split

- **Agent export** (tasks 1–3, sequential in one agent — shared blocks/mod)
- **Agent comments** (tasks 4–5, parallel with export)

## Verification

- Export a page with headings, **bold**, *italic*, `code`, code fence, list, quote, and an uploaded image → open DOCX in Word/LibreOffice and PDF in browser; styles + image visible.
- Comment on text → yellow mark visible → click mark → sidebar opens on that thread → reply → reply shows nested under parent.
- Resolve/delete still work; deleting root removes replies (cascade).

## Out of scope

- Nested replies deeper than 1 level (UI shows one reply level; backend can allow deeper but UI flattens under root)
- Comment notifications / realtime
- Export of comment annotations into DOCX/PDF

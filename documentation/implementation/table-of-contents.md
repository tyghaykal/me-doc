# Table of contents (right sidebar)

## Goal

When a document is open, show a right-side table of contents listing H1–H6 headings from the TipTap editor. Clicking an entry scrolls to that heading; the active section highlights as the user scrolls.

## Approach

- New `TableOfContents.vue` component — light outline (left rail + active border), not a separate panel.
- Editor emits `editor-ready` with the TipTap instance so the page can pass it to the TOC.
- Page layout (`[[pageId]].vue`): TOC is a sticky sibling **inside** the main document scroll surface (flex next to the editor), not a shell column. Same canvas as the content.
- Headings are derived from `editor.state.doc` on `update`; scroll target uses `editor.view.nodeDOM(pos)`.
- Active heading is the last heading whose top is above ~15% of the editor scroll container.
- Thin scrollbars via `.thin-scrollbar` utility in `main.css`.

## Files

- `frontend/app/components/TableOfContents.vue` — new
- `frontend/app/components/Editor.vue` — expose `editor`
- `frontend/app/pages/app/[[pageId]].vue` — layout + mount TOC

## Out of scope

- Collapse/expand nested outline tree (flat indented list is enough)
- Manual reordering of headings
- Mobile TOC drawer

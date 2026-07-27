# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

Anyone who registers an account on the deployment — the operator runs one instance of
me-doc (via Docker Compose, on their own server) and opens registration to the general
public, similar to a free SaaS product. Users are not expected to self-host or run
infrastructure themselves; they just sign up and use it. No confirmed narrower audience
(e.g. a specific company or team type) beyond "anyone who registers."

## Product Purpose

A Notion-style collaborative docs app: nested page trees, a real-time rich-text editor,
sharing/permissions, version history, and export to Markdown/DOCX/PDF. Success is a team
or individual being able to write and organize documents together, live, without losing
ownership of their data.

## Positioning

Free to use, and privacy-first: user data is confined to the document owner and is not
used, sold, or accessed by anyone else (including the operator) beyond what's needed to
run the service — distinct from SaaS competitors (Notion, Confluence, Outline) that hold
customer data on shared multi-tenant infrastructure with broader internal access/usage
rights. Live collaborative editing — of both documents and diagrams (Mermaid diagrams
co-edited in real time as first-class pages) — is the other core strength; this is a
genuinely different editing mechanism, not just a static embed.

## Operating Context

- Nested workspaces and page trees; drag-and-drop reparent/reorder; trash + restore.
- Real-time multi-user editing over WebSocket (Yjs CRDT), including live remote carets/presence.
- Sharing via email invite (viewer/editor) or public link tokens; recursive permission
  resolution (page → parents → workspace).
- Export to Markdown, DOCX, PDF; one version snapshot per finished editing session.
- In progress: diagrams-as-pages (Mermaid, text + drag-and-drop canvas, live co-editing,
  embeddable inline or standalone) and real-time comment fan-out over WebSocket.

## Capabilities and Constraints

- Auth: email + password, then a mandatory OTP second factor; JWT access tokens + rotating
  httpOnly refresh cookies.
- Collab access currently checks workspace membership, not fine-grained viewer/editor
  permission (known follow-up).
- Restoring a version while a live collab room is open doesn't push into that room until
  clients reconnect.
- Image embeds in DOCX/PDF export are alt-text only (no remote fetch/embed).
- Dev TLS is self-signed; not production TLS.
- Stack: Nuxt 4 / Vue 3 / Tiptap / Yjs frontend; Rust/axum / Postgres / Redis / MinIO backend.

## Brand Commitments

Name is "me-doc," tagline "write together, live." No logo beyond the current generic
favicon/apple-touch-icon; no other binding brand assets confirmed yet.

## Evidence on Hand

None. No testimonials, case studies, benchmarks, or press exist yet — future work must
not fabricate them.

## Product Principles

1. Data belongs to the document owner — never used, sold, or exposed beyond serving the product.
2. Real-time collaboration (documents and diagrams alike) is a first-class mechanism, not a bolt-on.
3. Free and open to anyone who registers — no self-hosting burden pushed onto the end user.
4. Users can always get their content out (Markdown/DOCX/PDF export, version history).
5. Reuse the existing page subsystem (CRUD, permissions, collab, search) for new content types rather than parallel systems.

## Accessibility & Inclusion

Target WCAG 2.1 AA.

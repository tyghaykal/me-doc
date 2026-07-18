# Tailwind + SSR/CSR rendering split (landing vs `/app`)

## Context

At Phase 1 (scaffolding), the Nuxt 4 frontend had only a stub `index.vue` (landing page) — no styling system and no `/app` route yet. The architecture doc already stated the app portion should call the Rust API directly rather than through Nuxt server routes (i.e. CSR was already the intent for the authenticated app, just never made explicit in config). This work made that rendering split concrete, added Tailwind, and recorded both decisions in `architecture.md`.

## Approach (as implemented)

Nuxt's built-in per-route rendering control (`routeRules` in `nuxt.config.ts`) handles the SSR/CSR split — no custom toggling needed.

For Tailwind, the community `@nuxtjs/tailwindcss` module was tried first but pulls in an incompatible Nuxt-3-era `@nuxt/kit`/jiti dependency chain that breaks under Nuxt 4 (`Cannot use 'import.meta' outside a module` at config-load time). Switched to Tailwind's own official `@tailwindcss/vite` plugin instead — first-party, no PostCSS config, no `tailwind.config.js`, just the Vite plugin plus one `@import "tailwindcss";` CSS entry.

1. **Tailwind** — `pnpm add -D tailwindcss @tailwindcss/vite`; registered via `vite.plugins: [tailwindcss()]` in `nuxt.config.ts`; CSS entry at `frontend/app/assets/css/main.css`. Utility classes applied to `index.vue` to prove the pipeline end-to-end.
2. **Rendering split** — `routeRules: { '/app/**': { ssr: false } }` in `nuxt.config.ts`. Landing page (`/`) needed no change — Nuxt's default universal (SSR) rendering already applies.
3. **Scaffolded `/app`** — `frontend/app/pages/app/index.vue`, a minimal placeholder. Used a directory (`pages/app/index.vue`) rather than a flat `pages/app.vue` so later phases (workspace dashboard, editor, etc.) can add nested routes under `/app/...` without restructuring.
4. **Updated `architecture.md`** — documented Tailwind as the styling approach and the explicit SSR-landing/CSR-app rendering split under "Frontend (Nuxt 4)".

## Files touched
- `frontend/package.json` / `pnpm-lock.yaml` — added `tailwindcss` + `@tailwindcss/vite` devDependencies
- `frontend/nuxt.config.ts` — added `vite.plugins`, `css`, `routeRules`
- `frontend/app/assets/css/main.css` — new, single `@import "tailwindcss";`
- `frontend/app/pages/index.vue` — Tailwind utility classes applied
- `frontend/app/pages/app/index.vue` — new placeholder CSR page
- `frontend/.nvmrc` — new, pins Node 22
- `documentation/implementation/architecture.md` — records the decision

## Unplanned but necessary: Node version bump

Verification surfaced a real, pre-existing blocker unrelated to Tailwind: this project's dependency resolution (`oxc-parser`/`oxc-walker`, used internally by Nuxt 4.4.8) requires Node's synchronous `require()` of ESM modules, which only Node 22+ supports. Under the previously-used Node 20.18.1, **both** `nuxt dev` and `nuxt build` crashed on any page — confirmed by reproducing the failure on a bare-bones `nuxt.config.ts` with no Tailwind/routeRules at all.

Fix: installed Node 22 via `nvm`, added `frontend/.nvmrc` pinning `22`, reinstalled dependencies. Anyone running this locally needs `nvm use` (or equivalent) picked up from `.nvmrc` now.

Also note: `pnpm` versions ≥10/11 enforce a `minimumReleaseAge` supply-chain policy that rejects very recently published packages. Under Node 22, `corepack` initially pulled pnpm 11.x and rejected Tailwind v4.3.3 as "too new" — resolved by installing the same pnpm version (9.15.9) already used elsewhere in the project instead of overriding the security policy.

## Verification performed
- `pnpm dev`: `curl http://localhost:3000/` returns full SSR HTML containing the applied Tailwind classes (`text-3xl font-bold text-slate-900`, etc.) — confirmed server-rendered, not just present in source.
- `curl http://localhost:3000/app` returns an empty `<div id="__nuxt"></div>` shell (200) — confirms CSR-only, no pre-rendered content.
- `pnpm build` completes with no errors, produces a working `.output/`.

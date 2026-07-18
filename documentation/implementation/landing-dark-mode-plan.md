# Landing Page Redesign + Dark Mode Default — Plan & Tasks

## Context

The current `/` route (`frontend/app/pages/index.vue`) is a 15-line placeholder that just dumps the backend `/health` response — never replaced since the initial SSR/CSR scaffolding work. The app also has zero dark-mode infrastructure (confirmed via a full grep of `app/` for `dark:`, `color-scheme`, `useColorMode` — no matches). The user wants:

1. An elegant, Notion-like marketing landing page (nav + hero + feature grid + footer — a **focused** scope: no pricing/testimonials/FAQ, since none of that exists in the actual product and would be filler).
2. Dark mode added and made the **default** theme app-wide, with a light-mode toggle available (confirmed: dark-default-with-toggle, not dark-only).

This is a frontend-only change; the backend is untouched. Two Explore agents mapped the current file inventory, palette, and confirmed there's no existing layout system (every page is self-contained) and no reusable icon/animation/color-mode dependency already installed. A third agent verified the exact Tailwind v4 dark-variant syntax and Nuxt `app.head` mechanics against the actually-installed package sources (not guessed).

## Design decisions

- **No new npm dependencies.** No icon library, no `@nuxtjs/color-mode`, no animation library. A `.dark`-class-based Tailwind v4 variant + a ~15-line composable + hand-written inline SVGs covers everything, consistent with this project's existing preference for small hand-rolled solutions over dependencies (e.g. the page tree's hand-rolled drag-and-drop).
- **No shared layout introduced.** The project has never had one; introducing it now would be an unrequested refactor of all 6 existing pages. The landing page instead composes a few small section components, matching the existing one-concern-per-file style (`ShareDialog.vue`, `PageTree.vue`).
- **Toggle placement:** landing nav + dashboard header only. The 4 auth pages (login/register/otp/login-otp) deliberately get no toggle — low-traffic transactional pages, out of scope per the "focused" instruction.
- **Accent color:** the current palette is 100% neutral `slate` grays with no hue anywhere. Introduce **indigo** as the single accent for primary CTAs — nothing else changes color.

## 1. Dark-mode infrastructure

**`frontend/app/assets/css/main.css`** — add the v4 custom-variant line (confirmed against the installed `tailwindcss@4.3.3` source):
```css
@import "tailwindcss";
@custom-variant dark (&:where(.dark, .dark *));
```
This makes every `dark:` utility key off a `.dark` class ancestor instead of the OS `prefers-color-scheme`.

**`frontend/nuxt.config.ts`** — add an `app.head` block so dark renders server-side with zero flash:
```ts
app: {
  head: {
    htmlAttrs: { class: 'dark' },
    script: [
      {
        innerHTML: `(function(){try{if(localStorage.getItem('theme')==='light')document.documentElement.classList.remove('dark')}catch(e){}})()`,
      },
    ],
  },
},
```
`innerHTML` + default `tagPosition: 'head'` (verified against `unhead`'s real type declarations) place this as a blocking script before `<body>` paints — it only ever *removes* `.dark` for the "user previously chose light" case; the default (no stored preference) needs no client JS at all since `htmlAttrs` already rendered it server-side. This only matters for the SSR'd routes (`/`, `/login`, `/register`, `/verify-otp`, `/login/otp`) — `/app/**` is already `ssr: false` per the existing `routeRules`, so there's no SSR/flash concern there at all.

**`frontend/app/composables/useTheme.ts`** (new) — one composable covers both SSR pages and the CSR-only dashboard:
```ts
export function useTheme() {
  const isDark = useState('theme-isDark', () => true) // matches the SSR default

  function apply(dark: boolean) {
    document.documentElement.classList.toggle('dark', dark)
    localStorage.setItem('theme', dark ? 'dark' : 'light')
    isDark.value = dark
  }

  onMounted(() => {
    // SSR pages: inline script already resolved the class pre-paint; this just
    // reads it back into reactive state. /app/** (ssr:false): nothing has run
    // yet, so this IS the resolution — mirrors the inline script's logic.
    const stored = localStorage.getItem('theme')
    apply(stored ? stored === 'dark' : true)
  })

  function toggleTheme() {
    apply(!isDark.value)
  }

  return { isDark, toggleTheme }
}
```
No Nuxt plugin file — the inline script (SSR pre-paint) plus this composable's `onMounted` is the complete mechanism; a plugin would just duplicate what `onMounted` already does. `useState` shares one reactive flag across every caller without needing a Pinia store for a single boolean.

**Toggle button** (reused wherever needed, ~8 lines, no icon library — hand-written inline SVG paths for sun/moon):
```vue
<button
  type="button"
  aria-label="Toggle theme"
  class="rounded p-1.5 text-slate-500 hover:bg-slate-100 dark:text-slate-400 dark:hover:bg-slate-800"
  @click="toggleTheme"
>
  <svg v-if="isDark" ...sun path.../>
  <svg v-else ...moon path.../>
</button>
```
Placed in: the new `LandingNav.vue`, and in `app/pages/app/index.vue`'s existing header action row (`<div class="flex items-center gap-2">`, alongside Export/History/Share/Logout).

## 2. Dark-variant retrofit pattern (existing 10 files, `index.vue` is replaced not retrofitted)

Apply this class-pairing pattern uniformly — no new decisions per file:

| Existing (light) | Add (dark) |
|---|---|
| `bg-white` | `dark:bg-slate-900` |
| `bg-slate-50` (page bg) | `dark:bg-slate-950` |
| `text-slate-900` / `text-slate-700` | `dark:text-slate-100` / `dark:text-slate-300` |
| `text-slate-400/500/600` (muted) | `dark:text-slate-500` / `dark:text-slate-400` |
| `border-slate-200/300` | `dark:border-slate-700` / `dark:border-slate-800` |
| `hover:bg-slate-50/100` | `dark:hover:bg-slate-800` |
| `bg-slate-900` (primary buttons) | `dark:bg-slate-100 dark:text-slate-900` (inverts so it stays high-contrast, doesn't disappear into the dark bg) |
| `text-green-600` / `text-red-600` | `dark:text-green-400` / `dark:text-red-400` |
| `bg-black/40` (modal overlay) | `dark:bg-black/60` |

**Worked example — `ShareDialog.vue`** (actual current markup):
```vue
<!-- before -->
<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4 font-sans">
  <div class="w-full max-w-md rounded-lg bg-white p-6 shadow-xl">
    <h2 class="text-xl font-bold text-slate-900">Share</h2>
    <input class="flex-1 rounded border border-slate-300 px-3 py-2 text-sm" />
    <button class="rounded bg-slate-900 px-3 py-2 text-sm font-medium text-white disabled:opacity-50">Invite</button>

<!-- after -->
<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 dark:bg-black/60 p-4 font-sans">
  <div class="w-full max-w-md rounded-lg bg-white dark:bg-slate-900 p-6 shadow-xl">
    <h2 class="text-xl font-bold text-slate-900 dark:text-slate-100">Share</h2>
    <input class="flex-1 rounded border border-slate-300 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100 px-3 py-2 text-sm" />
    <button class="rounded bg-slate-900 dark:bg-slate-100 px-3 py-2 text-sm font-medium text-white dark:text-slate-900 disabled:opacity-50">Invite</button>
```
The remaining files (`Editor.vue`, `ExportMenu.vue`, `PageTree.vue`, `VersionHistory.vue`, `app/pages/app/index.vue`, `login.vue`, `login/otp.vue`, `register.vue`, `verify-otp.vue`) follow this identical pairing — mechanical per file, all under 175 lines.

## 3. New landing page

```
frontend/app/pages/index.vue          — thin shell: <LandingNav/><LandingHero/><LandingFeatures/><LandingFooter/>
frontend/app/components/landing/LandingNav.vue       — logo + Login/Get Started links + theme toggle
frontend/app/components/landing/LandingHero.vue      — headline, subheadline, CTA → /register, secondary → /login
frontend/app/components/landing/LandingFeatures.vue  — 4-card grid, inline SVG icons
frontend/app/components/landing/LandingFooter.vue    — logo, copyright, repeat login/register links
```

Feature grid content (grounded in what's actually built, not aspirational):
1. **Real-time collaboration** — Yjs/CRDT-backed live co-editing
2. **Rich block editor** — Tiptap: headings, lists, code blocks, images
3. **Sharing & permissions** — public links + per-user viewer/editor roles
4. **Export anywhere** — Markdown, DOCX, PDF

Design language: dark-first (`bg-slate-950`/`bg-slate-900` surfaces, `text-slate-100` primary / `text-slate-400` muted text, `border-slate-800` dividers, thin borders instead of heavy shadows), indigo accent (`bg-indigo-600 hover:bg-indigo-500`, with `dark:` inverse pairing per the retrofit pattern) on the primary CTA only, generous section padding (`py-24`/`py-32`) — flat, content-forward, matches Notion's minimal aesthetic.

## 4. Verification plan

1. `docker compose up -d frontend` (or `pnpm dev` in `frontend/`).
2. Fresh incognito load of `/` — renders fully dark, zero flash of light background.
3. Click the nav toggle → flips to light, `localStorage.theme === 'light'`; reload → loads light with no flash (inline script removes `.dark` pre-paint).
4. Toggle back to dark, reload → persists.
5. Log in, go to `/app` — dashboard toggle works and agrees with the same `localStorage` key set from the landing page.
6. Spot-check both themes on: `/` (landing), `/app` (dashboard + open `ShareDialog` + visible `Editor`), `/login` (no toggle, but matches current theme).
7. `curl -I http://localhost:3000/` (or view-source) to confirm `/` still returns full SSR HTML (unregressed), and confirm `/app`'s initial HTML is still the empty CSR shell — `routeRules` behavior must be unchanged.

## Tasks

### Dark-mode infrastructure
- [x] **T1** Add `@custom-variant dark (&:where(.dark, .dark *));` to `frontend/app/assets/css/main.css`
- [x] **T2** Add `app.head.htmlAttrs`/`app.head.script` (default-dark + pre-paint light-preference removal) to `frontend/nuxt.config.ts` — *blocked by T1*
- [x] **T3** Create `frontend/app/composables/useTheme.ts` (`isDark`, `toggleTheme()`, `onMounted` sync) — *blocked by T1*
- [x] **T4** Add theme-toggle button markup (hand-written sun/moon SVGs) to `app/pages/app/index.vue`'s header action row — *blocked by T3*

### Dark-variant retrofit (existing files)
- [x] **T5** Retrofit `dark:` variants — `app/pages/app/index.vue` — *blocked by T3*
- [x] **T6** Retrofit `dark:` variants — `app/components/ShareDialog.vue` — *blocked by T1*
- [x] **T7** Retrofit `dark:` variants — `app/components/PageTree.vue` — *blocked by T1*
- [x] **T8** Retrofit `dark:` variants — `app/components/Editor.vue` — *blocked by T1*
- [x] **T9** Retrofit `dark:` variants — `app/components/ExportMenu.vue` — *blocked by T1*
- [x] **T10** Retrofit `dark:` variants — `app/components/VersionHistory.vue` — *blocked by T1*
- [x] **T11** Retrofit `dark:` variants — `app/pages/login.vue`, `login/otp.vue`, `register.vue`, `verify-otp.vue` (no toggle button on these, per scope cut) — *blocked by T1*

### New landing page
- [x] **T12** Build `app/components/landing/LandingNav.vue` (logo, Login/Get Started links, theme toggle) — *blocked by T3*
- [x] **T13** Build `app/components/landing/LandingHero.vue` (headline, subheadline, CTAs)
- [x] **T14** Build `app/components/landing/LandingFeatures.vue` (4-card grid: collab, editor, sharing, export)
- [x] **T15** Build `app/components/landing/LandingFooter.vue`
- [x] **T16** Replace `app/pages/index.vue` with the thin shell composing T12-T15 — *blocked by T12, T13, T14, T15*

### Verification
- [x] **T17** Live-verify: no-flash dark default, toggle + persistence across reload, `/app` dashboard toggle, SSR/CSR routeRules unregressed (steps 1-7 above) — *blocked by T2, T4, T16*

## Verification results

Verified live against the running `docker compose` stack (frontend restarted to pick up all changes, no compile/HMR errors on any of the ~15 touched files):
- `<html class="dark">` confirmed present in the server-rendered HTML for `/` — dark-by-default with no client JS needed on the happy path.
- Compiled Tailwind output (`/_nuxt/assets/css/main.css`) contains real `.dark` selectors — the `@custom-variant` directive was accepted and generated rules correctly.
- All routes return 200: `/`, `/app`, `/login`, `/register`, `/verify-otp`.
- Landing page content confirmed present in the rendered HTML: nav (Log in / Get started), hero, all 4 feature cards (Real-time collaboration, Rich block editor, Sharing & permissions, Export anywhere), footer.
- Not verified in this pass (needs a real browser, not curl): the toggle-to-light-and-persist round trip and visual spot-check of both themes. Recommend a quick manual check in-browser before considering this fully done.

## Known scope cuts
- No toggle on the 4 auth pages (deliberate).
- No new dependencies (icon lib, color-mode module, animation lib).
- No shared layout system introduced.

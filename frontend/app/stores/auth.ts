import { defineStore } from 'pinia'
import { appendResponseHeader } from 'h3'

interface User {
  id: string
  email: string
  display_name?: string | null
  avatar_key?: string | null
}

interface Workspace {
  id: string
  name: string
  slug: string
}

interface AuthResponse {
  access_token: string
  user: User
  workspace: Workspace | null
}

export const useAuthStore = defineStore('auth', () => {
  const accessToken = ref<string | null>(null)
  const user = ref<User | null>(null)
  const workspace = ref<Workspace | null>(null)

  const isAuthenticated = computed(() => accessToken.value !== null)

  function auth<T>(path: string, body?: unknown, headers?: HeadersInit) {
    return $fetch<T>(`${useApiBase()}/auth${path}`, {
      method: 'POST',
      credentials: 'include',
      headers,
      body,
    })
  }

  function applySession(res: AuthResponse) {
    accessToken.value = res.access_token
    user.value = res.user
    workspace.value = res.workspace
  }

  // Login/register responses only carry {id, email} — fetch the full profile
  // (display_name/avatar_key) so it's ready before any page that broadcasts
  // presence (Editor.vue builds its awareness `user` object synchronously at
  // setup time, so this must resolve before that component ever mounts).
  async function fetchMe() {
    if (!user.value) return
    const me = await $fetch<{ id: string; email: string; display_name: string | null; avatar_key: string | null }>(
      `${useApiBase()}/auth/me`,
      { headers: { Authorization: `Bearer ${accessToken.value}` } },
    )
    user.value = { ...user.value, display_name: me.display_name, avatar_key: me.avatar_key }
  }

  function register(email: string, password: string) {
    return auth<{ message: string }>('/register', { email, password })
  }

  async function verifyRegister(email: string, code: string) {
    applySession(await auth<AuthResponse>('/register/verify', { email, code }))
    await fetchMe()
  }

  function login(email: string, password: string) {
    return auth<{ message: string }>('/login', { email, password })
  }

  async function verifyLogin(email: string, code: string) {
    applySession(await auth<AuthResponse>('/login/verify', { email, code }))
    await fetchMe()
  }

  // `headers` lets SSR-side callers (e.g. middleware/guest.ts) forward the
  // incoming request's cookie by hand — there's no browser cookie jar during
  // SSR, so `credentials: 'include'` alone only works client-side.
  //
  // The backend rotates refresh tokens on every use (single-use, revoked on
  // consumption), so the rotated Set-Cookie from this call MUST be forwarded
  // back to the real browser response during SSR — otherwise the browser
  // keeps its now-revoked cookie and the next check (e.g. the dashboard's
  // own client-side refresh) fails, bouncing the user right back to /login.
  async function refresh(headers?: HeadersInit) {
    // Must capture the request event BEFORE the await below — Nuxt's async
    // context doesn't survive crossing an await from inside a Pinia store,
    // so calling useRequestEvent() after the fetch resolves throws "called
    // outside of a plugin/middleware/setup" instead of returning the event.
    const event = import.meta.server ? useRequestEvent() : undefined

    const response = await $fetch.raw<AuthResponse>(`${useApiBase()}/auth/refresh`, {
      method: 'POST',
      credentials: 'include',
      headers,
    })

    const setCookie = response.headers.get('set-cookie')
    if (setCookie && event) appendResponseHeader(event, 'set-cookie', setCookie)

    applySession(response._data!)
    await fetchMe()
  }

  async function logout() {
    await auth<{ message: string }>('/logout')
    accessToken.value = null
    user.value = null
    workspace.value = null
  }

  return {
    accessToken,
    user,
    workspace,
    isAuthenticated,
    register,
    verifyRegister,
    login,
    verifyLogin,
    refresh,
    logout,
    fetchMe,
  }
})

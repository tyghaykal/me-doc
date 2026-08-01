import { toValue, type MaybeRefOrGetter } from 'vue'
import { useCommentsStore, type CommentEvent } from '~/stores/comments'

/** Pushed by `pages::update_page` (backend) whenever title/icon change. */
interface PageMetaEvent {
  type: 'page'
  title: string
  icon: string | null
}

/**
 * Keeps the comments store live for the open page: subscribes to
 * `/ws/comments/:id` and applies every create/reply/resolve/delete pushed by
 * the backend, so a user always sees the current comment thread without
 * polling. Re-seeds via `fetchComments` on each (re)connect to close the gap
 * between the last fetch and the socket opening.
 *
 * This socket is already open for every viewer of a page regardless of
 * whether the comments sidebar is open, so it doubles as the transport for
 * title/icon rename events — those live in Postgres, not the Yjs doc, so they
 * don't otherwise ride the collab WebSocket's own realtime sync. `onPageMeta`
 * lets the caller patch state the pages store doesn't hold itself (e.g. an
 * anonymous link-guest's standalone `sharedPage`, not part of `pages.value`).
 */
export function useCommentStream(
  pageId: MaybeRefOrGetter<string | null | undefined>,
  linkToken?: MaybeRefOrGetter<string | null | undefined>,
  onPageMeta?: (patch: { title: string; icon: string | null }) => void,
) {
  const auth = useAuthStore()
  const store = useCommentsStore()
  const pagesStore = usePagesStore()
  const wsBase = useApiBase().replace(/^http/, 'ws')

  let ws: WebSocket | null = null
  let reconnect: ReturnType<typeof setTimeout> | null = null
  let attempt = 0
  let closedByUs = false

  function disconnect() {
    closedByUs = true
    if (reconnect) {
      clearTimeout(reconnect)
      reconnect = null
    }
    ws?.close()
    ws = null
  }

  function connect(id: string) {
    if (!import.meta.client) return
    const token = auth.accessToken
    const link = toValue(linkToken)
    // No credentials → nothing to authenticate the socket with; the sidebar's
    // REST fetch still works for public-link viewing without live updates.
    if (!token && !link) return

    closedByUs = false
    const params = new URLSearchParams()
    if (token) params.set('token', token)
    if (link) params.set('link', link)
    ws = new WebSocket(`${wsBase}/ws/comments/${id}?${params.toString()}`)

    ws.onopen = () => {
      attempt = 0
      // Seed from REST so we don't miss anything that changed while offline.
      store.fetchComments(id).catch(() => {})
    }
    ws.onmessage = (e) => {
      try {
        const data = JSON.parse(e.data) as CommentEvent | PageMetaEvent
        if (data.type === 'page') {
          pagesStore.patchPageMeta(id, { title: data.title, icon: data.icon })
          onPageMeta?.({ title: data.title, icon: data.icon })
        } else {
          store.applyEvent(data)
        }
      } catch {
        // Ignore malformed frames.
      }
    }
    ws.onclose = () => {
      ws = null
      if (closedByUs) return
      // Backoff reconnect: 1s, 2s, 4s … capped at 15s.
      const delay = Math.min(1000 * 2 ** attempt++, 15000)
      reconnect = setTimeout(() => connect(id), delay)
    }
  }

  watch(
    () => toValue(pageId),
    (id) => {
      disconnect()
      if (id) connect(id)
    },
    { immediate: true },
  )

  onBeforeUnmount(disconnect)
}

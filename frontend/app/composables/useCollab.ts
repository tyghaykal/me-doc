import * as Y from 'yjs'
import { WebsocketProvider } from 'y-websocket'

export interface PresenceUser {
  clientId: number
  name: string
  email: string | null
  color: string
  avatarUrl: string | null
}

/** Stable-ish HSL color from an id, so a user keeps the same cursor color. */
export function userColor(id: string): string {
  let hash = 0
  for (let i = 0; i < id.length; i++) hash = (hash * 31 + id.charCodeAt(i)) | 0
  return `hsl(${Math.abs(hash) % 360}, 70%, 50%)`
}

interface CollabOptions {
  pageId: string
  linkToken?: string | null
  /** Announce this client in awareness (shows up in peers' presence). Off for
   *  read-only consumers like an embed that only mirrors a diagram's content. */
  announce?: boolean
}

/**
 * Shared Yjs collaboration for a page id: a `Y.Doc` synced over the same
 * `/ws/pages/:id` room the rich-text editor uses, plus a live presence list.
 * The diagram editor stores its Mermaid source in this doc's `Y.Text('source')`;
 * the embed uses it read-only to mirror another diagram.
 */
export function useCollab(opts: CollabOptions) {
  const auth = useAuthStore()
  const config = useRuntimeConfig()
  const wsBase = (config.public.apiBase as string).replace(/^http/, 'ws')

  const doc = new Y.Doc()
  const params: Record<string, string> = {}
  if (auth.accessToken) params.token = auth.accessToken
  if (opts.linkToken) params.link = opts.linkToken
  const provider = new WebsocketProvider(`${wsBase}/ws/pages`, opts.pageId, doc, { params })

  const currentUser = {
    name: auth.user?.display_name || auth.user?.email || 'Anonymous',
    email: auth.user?.email ?? null,
    color: userColor(auth.user?.id ?? opts.pageId),
    avatarUrl: resolveAvatarUrl(auth.user?.avatar_key),
  }

  const presence = ref<PresenceUser[]>([])
  function updatePresence() {
    presence.value = Array.from(provider.awareness.getStates().entries())
      .filter(([clientId]) => clientId !== provider.awareness.clientID)
      .map(([clientId, state]: [number, any]) => ({
        clientId,
        name: state?.user?.name ?? 'Anonymous',
        email: state?.user?.email ?? null,
        color: state?.user?.color ?? '#999999',
        avatarUrl: state?.user?.avatarUrl ?? null,
      }))
  }

  const announce = opts.announce !== false
  if (announce) {
    provider.awareness.setLocalStateField('user', currentUser)
    provider.awareness.on('change', updatePresence)
  }

  function destroy() {
    if (announce) provider.awareness.off('change', updatePresence)
    provider.destroy()
    doc.destroy()
  }
  onBeforeUnmount(destroy)

  return { doc, provider, presence, currentUser, destroy }
}

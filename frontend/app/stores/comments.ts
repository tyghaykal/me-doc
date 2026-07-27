import { defineStore } from 'pinia'

export interface Comment {
  id: string
  page_id: string
  mark_id: string
  parent_id: string | null
  author_id: string
  author_email: string
  author_display_name: string | null
  assignee_id: string | null
  assignee_email: string | null
  assignee_display_name: string | null
  body: string
  resolved: boolean
  created_at: string
}

/** Prefer profile name when set; fall back to email. */
export function commentAuthorLabel(c: Pick<Comment, 'author_display_name' | 'author_email'>): string {
  const name = c.author_display_name?.trim()
  return name || c.author_email
}

export function commentAssigneeLabel(
  c: Pick<Comment, 'assignee_display_name' | 'assignee_email'>,
): string | null {
  if (!c.assignee_email && !c.assignee_display_name) return null
  const name = c.assignee_display_name?.trim()
  return name || c.assignee_email
}

/** Server-pushed comment events (see backend `comments::CommentEvent`). */
export type CommentEvent =
  | { type: 'created'; comment: Comment }
  | { type: 'updated'; comment: Comment }
  | { type: 'deleted'; id: string }

export const useCommentsStore = defineStore('comments', () => {
  const api = useApi()

  const comments = ref<Comment[]>([])

  // Upsert by id so an optimistic local insert and the same row echoed back
  // over the realtime socket converge to one entry instead of duplicating.
  function upsert(comment: Comment) {
    const idx = comments.value.findIndex((c) => c.id === comment.id)
    if (idx !== -1) comments.value[idx] = comment
    else comments.value.push(comment)
  }

  function removeById(id: string) {
    // Drop the row and any replies that hung off it (backend cascades).
    comments.value = comments.value.filter((c) => c.id !== id && c.parent_id !== id)
  }

  /** Apply a realtime event from another (or the same) client. */
  function applyEvent(ev: CommentEvent) {
    if (ev.type === 'deleted') removeById(ev.id)
    else upsert(ev.comment)
  }

  async function fetchComments(pageId: string) {
    comments.value = await api<Comment[]>(`/pages/${pageId}/comments`)
  }

  async function addComment(
    pageId: string,
    markId: string,
    body: string,
    assigneeEmail?: string,
  ) {
    const comment = await api<Comment>(`/pages/${pageId}/comments`, {
      method: 'POST',
      body: {
        mark_id: markId,
        body,
        assignee_email: assigneeEmail || undefined,
      },
    })
    upsert(comment)
    return comment
  }

  async function addReply(pageId: string, parentId: string, body: string) {
    const comment = await api<Comment>(`/pages/${pageId}/comments`, {
      method: 'POST',
      body: {
        parent_id: parentId,
        body,
      },
    })
    upsert(comment)
    return comment
  }

  async function resolveComment(id: string) {
    const updated = await api<Comment>(`/comments/${id}/resolve`, { method: 'PATCH' })
    upsert(updated)
    return updated
  }

  async function deleteComment(id: string) {
    await api(`/comments/${id}`, { method: 'DELETE' })
    removeById(id)
  }

  function roots(): Comment[] {
    return comments.value.filter((c) => !c.parent_id)
  }

  function repliesOf(parentId: string): Comment[] {
    return comments.value.filter((c) => c.parent_id === parentId)
  }

  function byMarkId(markId: string): Comment | undefined {
    return comments.value.find((c) => !c.parent_id && c.mark_id === markId)
  }

  return {
    comments,
    applyEvent,
    fetchComments,
    addComment,
    addReply,
    resolveComment,
    deleteComment,
    roots,
    repliesOf,
    byMarkId,
  }
})

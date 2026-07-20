import { defineStore } from 'pinia'

export interface Workspace {
  id: string
  name: string
  slug: string
}

export interface WorkspaceMember {
  user_id: string
  email: string
  role: string
}

export const useWorkspacesStore = defineStore('workspaces', () => {
  const api = useApi()
  const list = ref<Workspace[]>([])

  async function fetchAll() {
    list.value = await api<Workspace[]>('/workspaces')
    return list.value
  }

  async function create(name: string) {
    const ws = await api<Workspace>('/workspaces', { method: 'POST', body: { name } })
    list.value.push(ws)
    return ws
  }

  const fetchMembers = (workspaceId: string) =>
    api<WorkspaceMember[]>(`/workspaces/${workspaceId}/members`)

  const addMember = (workspaceId: string, email: string, role: string) =>
    api<WorkspaceMember>(`/workspaces/${workspaceId}/members`, {
      method: 'POST',
      body: { email, role },
    })

  const removeMember = (workspaceId: string, userId: string) =>
    api(`/workspaces/${workspaceId}/members/${userId}`, { method: 'DELETE' })

  function setActive(ws: Workspace) {
    useAuthStore().workspace = ws
    if (import.meta.client) localStorage.setItem('activeWorkspaceId', ws.id)
  }

  return { list, fetchAll, create, fetchMembers, addMember, removeMember, setActive }
})

export interface RecentEntry {
  id: string
  title: string
  icon: string | null
}

const CAP = 10
const STORAGE_KEY = 'recentPages'

export function pushRecent(list: RecentEntry[], entry: RecentEntry, cap = CAP): RecentEntry[] {
  return [entry, ...list.filter((e) => e.id !== entry.id)].slice(0, cap)
}

export function useRecents() {
  const recents = useState<RecentEntry[]>('recents', () => {
    if (!import.meta.client) return []
    try {
      const raw = localStorage.getItem(STORAGE_KEY)
      return raw ? JSON.parse(raw) : []
    } catch {
      return []
    }
  })

  function record(entry: RecentEntry) {
    recents.value = pushRecent(recents.value, entry)
    localStorage.setItem(STORAGE_KEY, JSON.stringify(recents.value))
  }

  return { recents, record }
}

if (import.meta.dev && import.meta.client) {
  const capped = Array.from({ length: 15 }, (_, i) => ({ id: String(i), title: String(i), icon: null }))
    .reduce<RecentEntry[]>((list, entry) => pushRecent(list, entry), [])
  console.assert(capped.length === CAP, 'useRecents: list should cap at', CAP)
  console.assert(capped[0].id === '14', 'useRecents: most recently pushed should be first')

  const deduped = pushRecent(
    pushRecent([], { id: '1', title: 'A', icon: null }),
    { id: '1', title: 'A (renamed)', icon: '🔥' },
  )
  console.assert(deduped.length === 1 && deduped[0].title === 'A (renamed)', 'useRecents: re-visiting a page should dedupe, not duplicate')
}

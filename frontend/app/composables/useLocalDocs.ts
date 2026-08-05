// Typed here rather than from lib.dom (same reasoning as local.vue): the File
// System Access API isn't in every TypeScript DOM lib version. Extends
// local.vue's minimal handle shape with the two permission methods needed to
// silently regain write access to a handle restored from IndexedDB.
export type FileHandle = {
  name: string
  getFile(): Promise<File>
  createWritable(): Promise<{ write(data: string): Promise<void>; close(): Promise<void> }>
  queryPermission(opts: { mode: 'readwrite' }): Promise<'granted' | 'denied' | 'prompt'>
  requestPermission(opts: { mode: 'readwrite' }): Promise<'granted' | 'denied' | 'prompt'>
}

export interface LocalDocEntry {
  /** The filename, which is also the identity — Save As under a new name is,
   *  from the filesystem's own point of view, a different file. */
  name: string
  savedAt: string
}

const CAP = 20
const STORAGE_KEY = 'localDocuments'

export function pushLocalDoc(list: LocalDocEntry[], entry: LocalDocEntry, cap = CAP): LocalDocEntry[] {
  return [entry, ...list.filter((e) => e.name !== entry.name)].slice(0, cap)
}

// --- IndexedDB: the only client storage that can hold a FileSystemFileHandle
// itself (structured-clone support) — cookies/localStorage only hold strings,
// so the visible "Local" list and the actual reopenable handle necessarily
// live in two different stores.
const DB_NAME = 'medoc-local-docs'
const DB_STORE = 'handles'

function openHandleDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, 1)
    req.onupgradeneeded = () => req.result.createObjectStore(DB_STORE)
    req.onsuccess = () => resolve(req.result)
    req.onerror = () => reject(req.error)
  })
}

async function putHandle(name: string, handle: FileHandle): Promise<void> {
  const db = await openHandleDb()
  await new Promise<void>((resolve, reject) => {
    const tx = db.transaction(DB_STORE, 'readwrite')
    tx.objectStore(DB_STORE).put(handle, name)
    tx.oncomplete = () => resolve()
    tx.onerror = () => reject(tx.error)
  })
}

async function getHandle(name: string): Promise<FileHandle | undefined> {
  const db = await openHandleDb()
  return new Promise((resolve, reject) => {
    const req = db.transaction(DB_STORE, 'readonly').objectStore(DB_STORE).get(name)
    req.onsuccess = () => resolve(req.result)
    req.onerror = () => reject(req.error)
  })
}

async function deleteHandle(name: string): Promise<void> {
  const db = await openHandleDb()
  await new Promise<void>((resolve, reject) => {
    const tx = db.transaction(DB_STORE, 'readwrite')
    tx.objectStore(DB_STORE).delete(name)
    tx.oncomplete = () => resolve()
    tx.onerror = () => reject(tx.error)
  })
}

export function useLocalDocs() {
  const docs = useState<LocalDocEntry[]>('localDocs', () => {
    if (!import.meta.client) return []
    try {
      const raw = localStorage.getItem(STORAGE_KEY)
      return raw ? JSON.parse(raw) : []
    } catch {
      return []
    }
  })

  function persist(next: LocalDocEntry[]) {
    docs.value = next
    if (import.meta.client) localStorage.setItem(STORAGE_KEY, JSON.stringify(next))
  }

  /** Call once per handle acquisition (Open, Save As) — not on every autosave
   *  tick, since the handle itself doesn't change between saves. */
  async function record(name: string, handle: FileHandle) {
    persist(pushLocalDoc(docs.value, { name, savedAt: new Date().toISOString() }))
    await putHandle(name, handle).catch(() => {})
  }

  /** Resolves a sidebar entry back into a live, write-ready handle. Re-granting
   *  permission needs a user gesture on a fresh page load — satisfied by
   *  whatever click led here. Returns null if the handle is gone or access was
   *  denied, so the caller can drop the now-stale entry. */
  async function open(name: string): Promise<FileHandle | null> {
    const handle = await getHandle(name).catch(() => undefined)
    if (!handle) return null
    const opts = { mode: 'readwrite' as const }
    if ((await handle.queryPermission(opts)) === 'granted') return handle
    if ((await handle.requestPermission(opts)) === 'granted') return handle
    return null
  }

  function remove(name: string) {
    persist(docs.value.filter((e) => e.name !== name))
    void deleteHandle(name)
  }

  return { docs, record, open, remove }
}

if (import.meta.dev && import.meta.client) {
  const capped = Array.from({ length: 25 }, (_, i) => ({ name: String(i), savedAt: '' }))
    .reduce<LocalDocEntry[]>((list, entry) => pushLocalDoc(list, entry), [])
  console.assert(capped.length === CAP, 'useLocalDocs: list should cap at', CAP)
  console.assert(capped[0].name === '24', 'useLocalDocs: most recently pushed should be first')

  const deduped = pushLocalDoc(
    pushLocalDoc([], { name: 'a.md', savedAt: '1' }),
    { name: 'a.md', savedAt: '2' },
  )
  console.assert(deduped.length === 1 && deduped[0].savedAt === '2', 'useLocalDocs: re-saving should dedupe, not duplicate')
}

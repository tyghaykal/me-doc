import type { FetchOptions } from 'ofetch'

export function useApi() {
  const auth = useAuthStore()
  const base = useApiBase()

  return function api<T>(path: string, options: FetchOptions = {}): Promise<T> {
    const request = (): Promise<T> =>
      $fetch<T>(`${base}${path}`, {
        ...options,
        credentials: 'include',
        headers: {
          ...options.headers,
          ...(auth.accessToken ? { Authorization: `Bearer ${auth.accessToken}` } : {}),
        },
      } as FetchOptions) as Promise<T>

    return request().catch(async (err) => {
      if (err?.response?.status !== 401) throw err
      try {
        await auth.refresh()
      } catch {
        throw err
      }
      return request()
    })
  }
}

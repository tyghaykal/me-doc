// Whether the backend has Google sign-in configured — fetched from
// `/auth/google/status` so `GoogleButton` can be hidden instead of 503ing on
// click. Failure (network/backend down) defaults to hidden, not shown.
export function useGoogleAuthEnabled() {
  const base = useApiBase()
  return useAsyncData(
    'google-auth-enabled',
    () => $fetch<{ enabled: boolean }>(`${base}/auth/google/status`).then((r) => r.enabled),
    { default: () => false },
  )
}

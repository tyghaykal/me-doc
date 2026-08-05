// Global AI status, shared across components (BlockMenu, ...) via module-level
// state so the toast survives the popup that triggered it closing. A request
// that runs while the user has already dismissed the menu still shows its
// spinner → success/failure in the bottom-right corner.
export type AiTokenUsage = { prompt: number; completion: number; total: number } | null

export type AiStatus =
  | { kind: 'idle' }
  | { kind: 'loading'; label?: string }
  | { kind: 'success'; message: string; tokens: AiTokenUsage }
  | { kind: 'error'; message: string }

const status = ref<AiStatus>({ kind: 'idle' })
let dismissTimer: ReturnType<typeof setTimeout> | undefined

const SUCCESS_MS = 4000
const ERROR_MS = 6000

export function useAiStatus() {
  function dismiss() {
    clearTimeout(dismissTimer)
    status.value = { kind: 'idle' }
  }

  function start(label?: string) {
    clearTimeout(dismissTimer)
    status.value = { kind: 'loading', label }
  }

  function succeed(message: string, tokens: AiTokenUsage = null) {
    status.value = { kind: 'success', message, tokens }
    dismissTimer = setTimeout(dismiss, SUCCESS_MS)
  }

  function fail(message: string) {
    status.value = { kind: 'error', message }
    dismissTimer = setTimeout(dismiss, ERROR_MS)
  }

  return { status, start, succeed, fail, dismiss }
}

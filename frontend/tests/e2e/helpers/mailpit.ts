// Host-published Mailpit UI port (compose default 8125; playwright.config.ts
// overrides MAILPIT_UI_HOST_PORT from the repo .env when it remaps ports).
const base = `http://localhost:${process.env.MAILPIT_UI_HOST_PORT ?? '8125'}`

async function json<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${base}${path}`, init)
  if (!res.ok) throw new Error(`Mailpit ${path} failed: ${res.status} ${res.statusText}`)
  return res.json() as Promise<T>
}

/**
 * Polls Mailpit for the newest OTP mail sent to `email` and returns its 6-digit code.
 * The message is deleted once read, so a second call for the same address cannot
 * return the previous code (register-then-login flows reuse one address).
 */
export async function pollForOtp(email: string, timeoutMs = 15_000): Promise<string> {
  const deadline = Date.now() + timeoutMs
  for (;;) {
    const query = `to:${email}`
    const { messages } = await json<{ messages: { ID: string }[] }>(
      `/api/v1/search?query=${encodeURIComponent(query)}`,
    )
    const id = messages[0]?.ID
    if (id) {
      const { Text } = await json<{ Text: string }>(`/api/v1/message/${id}`)
      await fetch(`${base}/api/v1/messages`, {
        method: 'DELETE',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ IDs: [id] }),
      })
      const code = Text.match(/\b\d{6}\b/)?.[0]
      if (code) return code
      throw new Error(`No 6-digit code in mail to ${email}: ${Text.slice(0, 200)}`)
    }
    if (Date.now() > deadline) throw new Error(`No mail for ${email} within ${timeoutMs}ms`)
    await new Promise((r) => setTimeout(r, 250))
  }
}

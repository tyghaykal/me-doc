import { test, expect, gotoHydrated, registerViaUi } from './fixtures'
import { uniqueEmail } from './helpers/ids'

// `browser.newContext()` bypasses the `use` block, so the TLS/baseURL options
// the shared page fixture gets for free have to be repeated here.
const CONTEXT_OPTIONS = { baseURL: 'https://localhost', ignoreHTTPSErrors: true }

test('an edit by one user reaches another user on the same page', async ({ browser }) => {
  test.slow()

  const emailA = uniqueEmail()
  const emailB = uniqueEmail()
  const contextA = await browser.newContext(CONTEXT_OPTIONS)
  const contextB = await browser.newContext(CONTEXT_OPTIONS)

  try {
    const pageA = await contextA.newPage()
    const pageB = await contextB.newPage()
    await registerViaUi(pageA, emailA)
    await registerViaUi(pageB, emailB)

    await pageA.getByRole('button', { name: '+ New page' }).click()
    await pageA.waitForURL(/\/app\/[0-9a-f-]{36}/)
    const pageUrl = pageA.url()

    const share = pageA.getByRole('dialog', { name: 'Share page' })
    await pageA.getByRole('button', { name: 'Private' }).click()
    await share.getByPlaceholder('name@example.com').fill(emailB)
    await share.getByLabel('Invite role').selectOption('editor')
    await share.getByRole('button', { name: 'Invite' }).click()
    await expect(share.getByText(`Shared with ${emailB}.`)).toBeVisible()
    await share.getByRole('button', { name: 'Close' }).click()

    await gotoHydrated(pageB, pageUrl)
    const editorA = pageA.locator('.ProseMirror')
    const editorB = pageB.locator('.ProseMirror')
    await expect(editorA).toBeVisible()
    await expect(editorB).toBeVisible()

    const text = `synced text ${Date.now()}`
    await editorA.click()
    await editorA.pressSequentially(text)

    await expect(editorB).toContainText(text, { timeout: 20_000 })

    // Awareness: B's avatar (titled with their name, i.e. their email) in A's topbar.
    await expect(pageA.locator(`[title="${emailB}"]`).first()).toBeVisible({ timeout: 20_000 })
  } finally {
    await contextA.close()
    await contextB.close()
  }
})

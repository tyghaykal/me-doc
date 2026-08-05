import { test, expect, gotoHydrated } from './fixtures'

// The File System Access API's picker is a native OS dialog Playwright can't
// drive — mocked here with an in-memory handle so the real autosave/record
// code paths (createWritable/write/close, queryPermission) still run for
// real. Installed before the SPA's first real navigation (gotoHydrated does
// a genuine page.goto, so addInitScript's re-injection on that load applies).
async function mockFileSystemAccess(page: import('@playwright/test').Page) {
  await page.addInitScript(() => {
    ;(window as any).__writes = []
    let content = ''
    const handle = {
      name: 'mock.md',
      async getFile() {
        return new File([content], handle.name, { type: 'text/markdown' })
      },
      async createWritable() {
        return {
          async write(data: string) {
            content = data
            ;(window as any).__writes.push(data)
          },
          async close() {},
        }
      },
      async queryPermission() {
        return 'granted'
      },
      async requestPermission() {
        return 'granted'
      },
    }
    ;(window as any).showSaveFilePicker = async (opts: { suggestedName?: string }) => {
      if (opts?.suggestedName) handle.name = opts.suggestedName
      return handle
    }
    ;(window as any).showOpenFilePicker = async () => [handle]
  })
}

test('autosaves to the file handle after a pause, and lists the document under Local', async ({ authedPage }) => {
  await mockFileSystemAccess(authedPage)
  await gotoHydrated(authedPage, '/app/local')

  await authedPage.locator('input[placeholder="Untitled"]').fill('Autosave Test')
  await authedPage.locator('.ProseMirror').click()
  await authedPage.locator('.ProseMirror').pressSequentially('first line')

  await authedPage.getByRole('button', { name: 'Save As' }).click()
  await expect(authedPage.getByText('Saved', { exact: true })).toBeVisible()
  const firstWrites = await authedPage.evaluate(() => (window as any).__writes.length)
  expect(firstWrites).toBeGreaterThan(0)

  // Sidebar picks up the recorded entry (localStorage-backed, same render
  // path as Recents) without a reload.
  await expect(authedPage.getByText('Local', { exact: true })).toBeVisible()
  await expect(authedPage.getByRole('button', { name: 'Autosave Test.md' })).toBeVisible()

  // Further edits should autosave on their own after the debounce, with no
  // explicit Save click.
  await authedPage.locator('.ProseMirror').pressSequentially(' and more')
  await expect(authedPage.getByText('Unsaved changes')).toBeVisible()
  await expect(authedPage.getByText('Saved', { exact: true })).toBeVisible({ timeout: 5000 })
  const secondWrites = await authedPage.evaluate(() => (window as any).__writes.length)
  expect(secondWrites).toBeGreaterThan(firstWrites)
  const lastWrite = await authedPage.evaluate(() => (window as any).__writes.at(-1))
  expect(lastWrite).toContain('first line and more')
})

test('a Local entry whose handle can no longer be resolved is dropped with a clear error', async ({
  authedPage,
}) => {
  // Simulate a stale entry: listed in localStorage (as a real save would
  // leave it) but with no matching IndexedDB handle — e.g. a different
  // browser profile, or the handle's permission was revoked.
  await authedPage.addInitScript(() => {
    localStorage.setItem(
      'localDocuments',
      JSON.stringify([{ name: 'ghost.md', savedAt: new Date().toISOString() }]),
    )
  })
  await gotoHydrated(authedPage, '/app/local?open=ghost.md')

  await expect(authedPage.getByText(/Could not reopen "ghost\.md"/)).toBeVisible()
  // The stale entry is dropped from the sidebar, not left to fail again next time.
  await expect(authedPage.getByRole('button', { name: 'ghost.md' })).toHaveCount(0)
})

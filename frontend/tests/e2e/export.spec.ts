import { stat } from 'node:fs/promises'
import { test, expect, gotoHydrated, registerViaUi } from './fixtures'
import { uniqueEmail } from './helpers/ids'
import type { Page } from '@playwright/test'

const PAGE_URL = /\/app\/([0-9a-f-]{36})/

/** Creates a page from the sidebar and types a paragraph the export must contain. */
async function createPageWithContent(page: Page, text: string): Promise<string> {
  await page.getByRole('button', { name: '+ New page' }).click()
  await page.waitForURL(PAGE_URL, { timeout: 30_000 })

  const body = page.locator('.ProseMirror')
  await expect(body).toBeVisible()
  await body.click()
  await body.pressSequentially(text)
  await expect(body).toContainText(text)
  // Editor debounces its content PUT by 1500ms; export reads the saved doc.
  await page.waitForTimeout(3000)

  return page.url().match(PAGE_URL)![1]!
}

async function exportAs(page: Page, item: string, ext: string) {
  const downloadPromise = page.waitForEvent('download', { timeout: 60_000 })
  await page.getByRole('button', { name: 'Export' }).click()
  await page.getByRole('menuitem', { name: item }).click()

  const download = await downloadPromise
  expect(download.suggestedFilename()).toMatch(new RegExp(`\\.${ext}$`))
  const path = await download.path()
  expect((await stat(path)).size).toBeGreaterThan(20)
}

test('exports the open page as Markdown, Word and PDF', async ({ authedPage: page }) => {
  await createPageWithContent(page, 'Exportable paragraph for the e2e suite.')

  await exportAs(page, 'Markdown (.md)', 'md')
  await exportAs(page, 'Word (.docx)', 'docx')
  await exportAs(page, 'PDF (.pdf)', 'pdf')
})

test('hides the export menu from a viewer', async ({ authedPage: page, browser }) => {
  const viewerEmail = uniqueEmail()
  const viewerContext = await browser.newContext({
    baseURL: 'https://localhost',
    ignoreHTTPSErrors: true,
  })
  const viewerPage = await viewerContext.newPage()

  try {
    await registerViaUi(viewerPage, viewerEmail)

    const pageId = await createPageWithContent(page, 'Shared read-only content.')

    await page.getByRole('button', { name: 'Share', exact: true }).click()
    const dialog = page.getByRole('dialog', { name: 'Share page' })
    await dialog.getByPlaceholder('name@example.com').fill(viewerEmail)
    await dialog.getByLabel('Invite role').selectOption('viewer')
    await dialog.getByRole('button', { name: 'Invite' }).click()
    await expect(dialog.getByText(`Shared with ${viewerEmail}.`)).toBeVisible()

    await gotoHydrated(viewerPage, `/app/${pageId}`)
    await expect(viewerPage.getByText('Shared with you')).toBeVisible({ timeout: 30_000 })
    await expect(viewerPage.locator('.ProseMirror')).toContainText('Shared read-only content.')

    await expect(viewerPage.getByRole('button', { name: 'Export' })).toHaveCount(0)
    await expect(viewerPage.getByRole('menuitem', { name: 'Markdown (.md)' })).toHaveCount(0)
  } finally {
    await viewerContext.close()
  }
})

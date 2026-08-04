import { fileURLToPath } from 'node:url'
import { test, expect } from './fixtures'
import type { Page } from '@playwright/test'

const fixture = (name: string) => fileURLToPath(new URL(`./fixtures/${name}`, import.meta.url))

type UploadFile = string | { name: string; mimeType: string; buffer: Buffer }

/** Drives the sidebar "Import" button through its hidden file input. */
async function importFile(page: Page, file: UploadFile) {
  const chooser = page.waitForEvent('filechooser')
  await page.getByRole('button', { name: 'Import' }).click()
  await (await chooser).setFiles(file)
}

/** Import creates a page and makes it active, which pushes /app/<uuid>. */
async function waitForImportedPage(page: Page) {
  await page.waitForURL(/\/app\/[0-9a-f-]{36}/, { timeout: 30_000 })
}

test('imports a .md file into a new page', async ({ authedPage: page }) => {
  await importFile(page, fixture('sample.md'))
  await waitForImportedPage(page)

  await expect(page.locator('aside').getByText('sample', { exact: false })).toBeVisible()
  const body = page.locator('.ProseMirror')
  await expect(body).toContainText('Imported Heading')
  await expect(body).toContainText('A paragraph from a markdown fixture.')
  await expect(body.locator('li', { hasText: 'one' }).first()).toBeVisible()
})

test('imports a .txt file into a new page', async ({ authedPage: page }) => {
  await importFile(page, fixture('sample.txt'))
  await waitForImportedPage(page)

  const body = page.locator('.ProseMirror')
  await expect(body).toContainText('Plain text fixture line one.')
  await expect(body).toContainText('Plain text fixture line two.')
})

test('imports a .html file through the converter service', async ({ authedPage: page }) => {
  await importFile(page, fixture('sample.html'))
  // Round-trips through the backend + MarkItDown converter, so allow longer.
  await expect(page.getByRole('button', { name: 'Importing' })).toBeVisible()
  await waitForImportedPage(page)

  const body = page.locator('.ProseMirror')
  await expect(body).toContainText('Imported HTML Heading', { timeout: 30_000 })
  await expect(body).toContainText('A paragraph from an html fixture.')
})

test('rejects an unsupported file type without creating a page', async ({ authedPage: page }) => {
  // setFiles bypasses the input's `accept` filter, so this exercises the app's
  // own extension check rather than the browser's picker.
  await importFile(page, {
    name: 'malware.exe',
    mimeType: 'application/octet-stream',
    buffer: Buffer.from('MZ not a document'),
  })

  await expect(page.getByText('Unsupported file type.')).toBeVisible()
  await expect(page).toHaveURL(/\/app\/?$/)
  await expect(page.locator('aside').getByText('malware')).toHaveCount(0)
})

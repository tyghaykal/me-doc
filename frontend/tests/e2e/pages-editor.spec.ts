import type { Locator, Page } from '@playwright/test'
import { test, expect } from './fixtures'
import { uniqueName } from './helpers/ids'

/** The depth-0 PageTree list — the only one carrying the "+ New page" button. */
function privateTree(page: Page): Locator {
  return page.locator('aside ul').filter({ has: page.getByRole('button', { name: '+ New page' }) })
}

function treeRow(page: Page, title: string): Locator {
  return privateTree(page).getByText(title, { exact: true })
}

const titleInput = (page: Page) => page.getByPlaceholder('Untitled')
const editorBody = (page: Page) => page.locator('.ProseMirror')

/** Creates a page from the sidebar and renames it, waiting for the title PATCH. */
async function createPage(page: Page, title: string): Promise<void> {
  await page.getByRole('button', { name: '+ New page' }).click()
  await expect(titleInput(page)).toBeVisible()

  // The tree only picks up the new title once the rename PATCH resolves (no
  // debounce, but a real round trip) — wait for it instead of racing the
  // default assertion timeout under load.
  const renamed = page.waitForResponse(
    (r) => r.request().method() === 'PATCH' && new URL(r.url()).pathname === `/pages/${page.url().split('/app/')[1]!.split('?')[0]}` && r.ok(),
  )
  await titleInput(page).fill(title)
  await titleInput(page).blur()
  await renamed
  await expect(treeRow(page, title)).toBeVisible()
}

test('creates a page, renames it, and persists editor content across a reload', async ({
  authedPage: page,
}) => {
  const title = uniqueName('Notes')
  await createPage(page, title)
  await expect(page).toHaveURL(/\/app\/[0-9a-f-]{36}/)

  const body = 'Yjs round-trip content'
  // The content PUT is debounced 1.5s and dropped on unmount, so wait for the
  // real request before reloading rather than guessing at a sleep.
  const saved = page.waitForResponse(
    (r) => r.request().method() === 'PUT' && /\/pages\/[^/]+\/content/.test(r.url()) && r.ok(),
  )
  await editorBody(page).click()
  await editorBody(page).pressSequentially(body)
  await expect(editorBody(page)).toContainText(body)
  await saved

  await page.reload()
  await expect(editorBody(page)).toContainText(body)
  await expect(titleInput(page)).toHaveValue(title)
})

test('duplicates a page from the tree context menu', async ({ authedPage: page }) => {
  const title = uniqueName('Dup')
  await createPage(page, title)

  await treeRow(page, title).click({ button: 'right' })
  await page.getByRole('menuitem', { name: 'Duplicate' }).click()

  await expect(treeRow(page, `${title} (copy)`)).toBeVisible()
  await expect(treeRow(page, title)).toBeVisible()
})

test('trashes a page and lists it in the trash modal', async ({ authedPage: page }) => {
  const title = uniqueName('Trashed')
  await createPage(page, title)

  page.once('dialog', (d) => d.accept())
  await treeRow(page, title).click({ button: 'right' })
  await page.getByRole('menuitem', { name: 'Delete' }).click()
  await expect(treeRow(page, title)).toHaveCount(0)

  await page.getByRole('button', { name: 'Trash', exact: true }).click()
  const trash = page.getByRole('dialog', { name: 'Trash' })
  await expect(trash.getByText(title, { exact: true })).toBeVisible()

  await trash.getByRole('button', { name: 'Restore' }).click()
  await expect(treeRow(page, title)).toBeVisible()
})

test('finds a page by title through the search palette', async ({ authedPage: page }) => {
  const title = uniqueName('Findable')
  await createPage(page, title)
  await page.getByRole('button', { name: 'Home' }).click()

  await page.getByRole('button', { name: 'Search' }).click()
  const palette = page.getByRole('dialog', { name: 'Search' })
  await palette.getByPlaceholder('Search title, content, people…').fill(title)

  await palette.getByRole('button', { name: title }).click()
  await expect(palette).toBeHidden()
  await expect(titleInput(page)).toHaveValue(title)
})

import type { Page } from '@playwright/test'
import { test, expect } from './fixtures'

const PAGE_URL = /\/app\/[0-9a-f-]{36}/

async function createPage(page: Page, title: string): Promise<string> {
  await page.getByRole('button', { name: '+ New page' }).click()
  await page.waitForURL(PAGE_URL)
  await page.getByPlaceholder('Untitled').fill(title)
  await page.getByPlaceholder('Untitled').blur()
  return page.url().split('/app/')[1]!.split('?')[0]!
}

/** Types a paragraph, then adds a comment on it via the block context menu. */
async function addComment(page: Page, text: string, body: string): Promise<void> {
  const doc = page.locator('.ProseMirror')
  await doc.click()
  await page.keyboard.type(text)
  await expect(doc).toContainText(text)

  await doc.getByText(text).click({ button: 'right' })
  await page.getByRole('button', { name: 'Comment', exact: true }).click()
  await page.getByPlaceholder('Add a comment…').fill(body)
  await page.getByRole('button', { name: 'Comment', exact: true }).click()
}

function openSidebar(page: Page) {
  return page.getByRole('button', { name: 'Comments', exact: true }).click()
}

test('add, resolve and delete a comment', async ({ authedPage }) => {
  const page = authedPage
  await createPage(page, 'Comment target')
  await addComment(page, 'Needs review here', 'Please double-check this claim')

  await openSidebar(page)
  const sidebar = page.getByRole('complementary').filter({ hasText: 'Comments' })
  const card = sidebar.locator('li').filter({ hasText: 'Please double-check this claim' }).first()
  await expect(card).toBeVisible()

  await card.getByRole('button', { name: 'Resolve' }).click()
  await expect(card.getByRole('button', { name: 'Reopen' })).toBeVisible()
  await expect(card).toHaveClass(/opacity-60/)

  await card.getByRole('button', { name: 'Delete' }).click()
  await expect(sidebar.getByText('Please double-check this claim')).toHaveCount(0)
  await expect(sidebar.getByText('No comments yet.')).toBeVisible()
})

test('reply appears under its parent comment', async ({ authedPage }) => {
  const page = authedPage
  await createPage(page, 'Reply target')
  await addComment(page, 'Draft intro', 'Is this the final wording?')

  await openSidebar(page)
  const sidebar = page.getByRole('complementary').filter({ hasText: 'Comments' })
  const card = sidebar.locator('li').filter({ hasText: 'Is this the final wording?' }).first()

  await card.getByPlaceholder('Reply…').fill('Yes, ship it')
  await card.getByRole('button', { name: 'Reply' }).click()
  await expect(card.getByText('Yes, ship it')).toBeVisible()
})

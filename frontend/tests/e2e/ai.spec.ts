import type { Page } from '@playwright/test'
import { test, expect, gotoHydrated } from './fixtures'
import { uniqueName } from './helpers/ids'

async function createPageWithText(page: Page, text: string): Promise<void> {
  await page.getByRole('button', { name: '+ New page' }).click()
  await expect(page.getByPlaceholder('Untitled')).toBeVisible()
  await page.getByPlaceholder('Untitled').fill(uniqueName('AI'))
  await page.locator('.ProseMirror').click()
  await page.locator('.ProseMirror').pressSequentially(text)
}

test('AI settings round-trip, without ever echoing the key back', async ({ authedPage }) => {
  await gotoHydrated(authedPage, '/app/settings')

  await expect(authedPage.getByText('No key set yet.')).toBeVisible()

  await authedPage.getByLabel('API URL').fill('https://api.example.invalid/v1')
  await authedPage.getByLabel('API key').fill('sk-e2e-secret')
  await authedPage.getByLabel('Model').fill('gpt-4o-mini')
  await authedPage.getByRole('button', { name: 'Save' }).click()
  await expect(authedPage.getByText('Saved.')).toBeVisible()

  await gotoHydrated(authedPage, '/app/settings')
  await expect(authedPage.getByText('A key is currently set.')).toBeVisible()
  await expect(authedPage.getByLabel('API URL')).toHaveValue('https://api.example.invalid/v1')
  await expect(authedPage.getByLabel('Model')).toHaveValue('gpt-4o-mini')
  // The key is write-only: the field must come back blank, not pre-filled.
  await expect(authedPage.getByLabel('API key')).toHaveValue('')
})

test('AI actions (via the block menu) prompt for setup when no provider is configured', async ({ authedPage }) => {
  await createPageWithText(authedPage, 'the quick brown fox')

  // Same trigger comments.spec.ts already relies on: the block menu only
  // opens once the drag-handle extension has registered a hover.
  const target = authedPage.locator('.ProseMirror').getByText('the quick brown fox')
  await target.hover()
  await expect(authedPage.getByRole('button', { name: 'Open block menu' })).toBeVisible()
  await target.click({ button: 'right' })

  await authedPage.getByRole('button', { name: 'Ask AI' }).click()
  await authedPage.getByText('Reword, keeping its meaning').click()

  await expect(authedPage.getByText('Set up your AI provider to use this.')).toBeVisible()
  await expect(authedPage.getByRole('link', { name: 'Open settings' })).toHaveAttribute(
    'href',
    '/app/settings',
  )
  // The block itself must be left untouched when the request never happens.
  await expect(authedPage.locator('.ProseMirror')).toContainText('the quick brown fox')
})

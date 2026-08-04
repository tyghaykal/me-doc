import type { Locator, Page } from '@playwright/test'
import { test, expect, gotoHydrated, TEST_PASSWORD } from './fixtures'
import { uniqueName } from './helpers/ids'
import { pollForOtp } from './helpers/mailpit'

/** The UserMenu trigger's accessible name always contains the account email. */
const userMenu = (page: Page, email: string) => page.getByRole('button', { name: email })

async function openSettings(page: Page, email: string): Promise<Locator> {
  await userMenu(page, email).click()
  await page.getByRole('menuitem', { name: 'Update information' }).click()
  return page.getByRole('dialog', { name: 'User settings' })
}

/** Like the shared `loginViaUi`, but for an account whose password was rotated. */
async function loginWithPassword(page: Page, email: string, password: string): Promise<void> {
  await gotoHydrated(page, '/login')
  await page.getByLabel('Email').fill(email)
  await page.getByLabel('Password').fill(password)
  await page.getByRole('button', { name: 'Log in' }).click()
  await page.waitForURL('**/login-otp**')
  await page.getByLabel('Code').fill(await pollForOtp(email))
  await page.getByRole('button', { name: 'Verify' }).click()
  await page.waitForURL('**/app**')
}

test('display name is saved and survives a reload', async ({ authedPage, authedEmail }) => {
  const name = uniqueName('Tester')

  const dialog = await openSettings(authedPage, authedEmail)
  await dialog.getByLabel('Display name').fill(name)
  await dialog.getByRole('button', { name: 'Save' }).click()
  await expect(dialog.getByText('Saved.')).toBeVisible()
  await dialog.getByRole('button', { name: 'Close' }).click()

  await expect(authedPage.getByRole('button', { name })).toBeVisible()

  await gotoHydrated(authedPage, '/app')
  await expect(authedPage.getByRole('button', { name })).toBeVisible()
  await expect((await openSettings(authedPage, authedEmail)).getByLabel('Display name')).toHaveValue(
    name,
  )
})

test('password change rejects a wrong current password and the new one works', async ({
  authedPage,
  authedEmail,
}) => {
  test.slow()
  const newPassword = `${TEST_PASSWORD}-rotated`
  const dialog = await openSettings(authedPage, authedEmail)

  async function submitChange(currentPassword: string) {
    await dialog.getByLabel('Current password').fill(currentPassword)
    await dialog.getByLabel('New password', { exact: true }).fill(newPassword)
    await dialog.getByLabel('Confirm new password').fill(newPassword)
    await dialog.getByRole('button', { name: 'Change password' }).click()
  }

  await submitChange('definitely-not-the-password')
  await expect(dialog.getByText(/invalid email or password/i)).toBeVisible()
  await expect(dialog.getByText('Password changed.')).toHaveCount(0)

  await submitChange(TEST_PASSWORD)
  await expect(dialog.getByText('Password changed.')).toBeVisible()
  await dialog.getByRole('button', { name: 'Close' }).click()

  await userMenu(authedPage, authedEmail).click()
  await authedPage.getByRole('menuitem', { name: 'Log out' }).click()
  await authedPage.waitForURL('**/login**')

  await loginWithPassword(authedPage, authedEmail, newPassword)
})

test('creates a workspace, switches between workspaces, and lists members', async ({
  authedPage,
  authedEmail,
}) => {
  const wsName = uniqueName('Workspace')
  const switcher = (name: string) => authedPage.getByRole('button', { name })

  await switcher('My Workspace').click()
  await authedPage.getByRole('menuitem', { name: '+ New workspace' }).click()
  const create = authedPage.getByRole('dialog', { name: 'Create workspace' })
  await create.getByPlaceholder('Workspace name').fill(wsName)
  await create.getByRole('button', { name: 'Create' }).click()
  await expect(create).toBeHidden()

  // Creating switches to the new workspace; switch back and forth to prove both work.
  await expect(switcher(wsName)).toBeVisible()
  await switcher(wsName).click()
  await authedPage.getByRole('menuitem', { name: 'My Workspace' }).click()
  await expect(switcher('My Workspace')).toBeVisible()
  await switcher('My Workspace').click()
  await authedPage.getByRole('menuitem', { name: wsName }).click()
  await expect(switcher(wsName)).toBeVisible()

  await switcher(wsName).click()
  await authedPage.getByRole('menuitem', { name: 'Manage members' }).click()
  const members = authedPage.getByRole('dialog', { name: 'Workspace members' })
  await expect(members.getByText(authedEmail)).toBeVisible()
  await expect(members.getByText('owner')).toBeVisible()
})

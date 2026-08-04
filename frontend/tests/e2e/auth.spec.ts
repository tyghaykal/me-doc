import { test, expect, TEST_PASSWORD, gotoHydrated, loginViaUi, registerViaUi } from './fixtures'
import { uniqueEmail } from './helpers/ids'
import { pollForOtp } from './helpers/mailpit'

// The only error surface on the auth forms is a red <p> inside the form.
const formError = (page: import('@playwright/test').Page) => page.locator('form p.text-red-600')

test('registers, verifies the emailed OTP, and lands on /app', async ({ page }) => {
  const email = uniqueEmail()

  await gotoHydrated(page, '/register')
  await page.getByLabel('Email').fill(email)
  await page.getByLabel('Password').fill(TEST_PASSWORD)
  await page.getByRole('button', { name: 'Create account' }).click()

  await page.waitForURL('**/verify-otp**')
  await expect(page.getByRole('heading', { name: 'Verify your email' })).toBeVisible()

  await page.getByLabel('Code').fill(await pollForOtp(email))
  await page.getByRole('button', { name: 'Verify' }).click()

  await page.waitForURL('**/app**')
  await expect(page.getByRole('button', { name: '+ New page' })).toBeVisible()
})

test('rejects a wrong OTP without navigating away', async ({ page }) => {
  const email = uniqueEmail()

  await gotoHydrated(page, '/register')
  await page.getByLabel('Email').fill(email)
  await page.getByLabel('Password').fill(TEST_PASSWORD)
  await page.getByRole('button', { name: 'Create account' }).click()
  await page.waitForURL('**/verify-otp**')

  await page.getByLabel('Code').fill('000000')
  await page.getByRole('button', { name: 'Verify' }).click()

  await expect(formError(page)).toBeVisible()
  await expect(page).toHaveURL(/\/verify-otp/)

  // Non-numeric input is caught client-side before any request.
  await page.getByLabel('Code').fill('abc')
  await page.getByRole('button', { name: 'Verify' }).click()
  await expect(formError(page)).toHaveText('Enter the 6-digit code.')
  await expect(page).toHaveURL(/\/verify-otp/)
})

test('logs out and back in again', async ({ page }) => {
  const email = uniqueEmail()
  await registerViaUi(page, email)

  await page.getByRole('button', { name: email }).click()
  await page.getByRole('menuitem', { name: 'Log out' }).click()
  await page.waitForURL('**/login**')

  await loginViaUi(page, email)
  await expect(page.getByRole('button', { name: '+ New page' })).toBeVisible()
})

test('redirects an anonymous visitor from /app to /login', async ({ page }) => {
  await page.goto('/app')
  await page.waitForURL('**/login**')
  await expect(page.getByRole('heading', { name: 'Log in' })).toBeVisible()
})

test('lets an anonymous visitor reach /app with a ?link= token', async ({ page }) => {
  // The auth middleware exempts `?link=` so public share links resolve
  // server-side; a bogus token still renders the app route, not /login.
  await page.goto('/app/00000000-0000-0000-0000-000000000000?link=not-a-real-token')
  await expect(page.getByText("This link is invalid, expired, or you don't have access.")).toBeVisible()
  await expect(page).toHaveURL(/\/app\//)
})

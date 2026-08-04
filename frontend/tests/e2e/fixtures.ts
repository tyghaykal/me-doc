import { test as base, expect, type Page } from '@playwright/test'
import { uniqueEmail } from './helpers/ids'
import { pollForOtp } from './helpers/mailpit'

export const TEST_PASSWORD = 'test-password-123'

/**
 * Navigates and waits for Nuxt to mount the client app. Without this, a click on
 * a freshly-loaded page can fire before Vue attaches `@submit.prevent`, and the
 * form does a native submit that silently reloads the page instead.
 */
export async function gotoHydrated(page: Page, path: string): Promise<void> {
  await page.goto(path)
  await page.waitForFunction(() => !!(document.querySelector('#__nuxt') as any)?.__vue_app__)
}

/** Registers a brand-new account through the UI and verifies its emailed OTP. */
export async function registerViaUi(page: Page, email: string): Promise<void> {
  await gotoHydrated(page, '/register')
  await page.getByLabel('Email').fill(email)
  await page.getByLabel('Password').fill(TEST_PASSWORD)
  await page.getByRole('button', { name: 'Create account' }).click()

  await page.waitForURL('**/verify-otp**')
  await page.getByLabel('Code').fill(await pollForOtp(email))
  await page.getByRole('button', { name: 'Verify' }).click()
  await page.waitForURL('**/app**')
}

/** Logs an already-registered account back in through the UI (second OTP round-trip). */
export async function loginViaUi(page: Page, email: string): Promise<void> {
  await gotoHydrated(page, '/login')
  await page.getByLabel('Email').fill(email)
  await page.getByLabel('Password').fill(TEST_PASSWORD)
  await page.getByRole('button', { name: 'Log in' }).click()

  await page.waitForURL('**/login-otp**')
  await page.getByLabel('Code').fill(await pollForOtp(email))
  await page.getByRole('button', { name: 'Verify' }).click()
  await page.waitForURL('**/app**')
}

export const test = base.extend<{ authedPage: Page; authedEmail: string }>({
  authedEmail: async ({}, use) => {
    await use(uniqueEmail())
  },
  authedPage: async ({ page, authedEmail }, use) => {
    await registerViaUi(page, authedEmail)
    await use(page)
  },
})

export { expect }

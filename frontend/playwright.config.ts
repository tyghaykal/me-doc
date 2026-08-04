import { readFileSync } from 'node:fs'
import { defineConfig, devices } from '@playwright/test'

// The repo .env (gitignored) remaps the compose host ports on some machines, so
// helpers/mailpit.ts must use the same MAILPIT_UI_HOST_PORT the stack was started with.
try {
  for (const line of readFileSync(new URL('../.env', import.meta.url), 'utf8').split('\n')) {
    const m = line.match(/^\s*(MAILPIT_UI_HOST_PORT)\s*=\s*(.+?)\s*$/)
    if (m) process.env[m[1]!] ??= m[2]!
  }
} catch {
  // No .env — compose defaults apply.
}

export default defineConfig({
  testDir: './tests/e2e',
  fullyParallel: true,
  retries: 0,
  reporter: 'list',
  use: {
    baseURL: 'https://localhost',
    ignoreHTTPSErrors: true,
    trace: 'on-first-retry',
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
})

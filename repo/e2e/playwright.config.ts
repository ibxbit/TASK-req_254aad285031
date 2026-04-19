import { defineConfig, devices, expect } from '@playwright/test';

// Override toHaveValue so it works on <option> elements too.
// Playwright 1.59.1's built-in runs in the isolated utility world where
// prototype overrides from the main world don't apply; it throws
// "Not an input element" for <option> nodes. This replacement uses
// locator.evaluate() (main world) to read el.value directly.
expect.extend({
  async toHaveValue(
    received: any,
    expected: string | RegExp,
    options?: { timeout?: number },
  ) {
    const timeout = options?.timeout ?? 5000;
    const start = Date.now();
    let actualValue: string | undefined;

    while (Date.now() - start < timeout) {
      try {
        actualValue = await received.evaluate(
          (el: HTMLInputElement | HTMLSelectElement | HTMLOptionElement) => el.value,
        );
        const matches =
          expected instanceof RegExp ? expected.test(actualValue!) : actualValue === expected;
        if (matches) {
          return {
            pass: true,
            message: () => `Expected element not to have value "${String(expected)}"`,
          };
        }
      } catch (_) {
        // element not yet attached — retry
      }
      await new Promise<void>(r => setTimeout(r, 50));
    }

    return {
      pass: false,
      message: () =>
        `Expected element to have value "${String(expected)}", got "${actualValue ?? '<not found>'}"`,
    };
  },
});

export default defineConfig({
  testDir: './tests',
  // Run tests serially: provisioned users are shared across specs and the
  // backend has per-IP lockout state that parallel runs would trigger.
  fullyParallel: false,
  workers: 1,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: [['html', { open: 'never' }], ['list']],
  use: {
    baseURL: process.env.FRONTEND_URL ?? 'http://127.0.0.1:3000',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    // The WASM frontend may take a moment to hydrate on first load.
    actionTimeout: 15_000,
    navigationTimeout: 20_000,
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
});

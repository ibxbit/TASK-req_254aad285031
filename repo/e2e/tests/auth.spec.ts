/**
 * Auth flow E2E tests.
 *
 * Covers:
 *  - Login page structure (heading, inputs, button)
 *  - Successful login redirects to "/"
 *  - Invalid credentials show p.error message (class: "error" in Dioxus RSX)
 *  - Logout returns to /login and clears session
 *  - Unauthenticated access to protected routes shows "not signed in" or redirects
 *  - Session persistence: reload after login keeps the user logged in
 */

import { test, expect } from '@playwright/test';
import {
  loginAs,
  logout,
  ADMIN_USER,
  ADMIN_PASS,
  frontendReachable,
} from './helpers';

test.describe('Login page structure', () => {
  test.beforeEach(async ({ page }) => {
    if (!(await frontendReachable())) {
      test.skip(true, 'Frontend not reachable — start docker compose up first');
    }
    await page.goto('/login');
  });

  test('renders the sign-in heading', async ({ page }) => {
    await expect(page.locator('h1')).toContainText('Sign in');
  });

  test('has a username text input', async ({ page }) => {
    await expect(page.locator('input[type="text"]').first()).toBeVisible();
  });

  test('has a password input', async ({ page }) => {
    await expect(page.locator('input[type="password"]').first()).toBeVisible();
  });

  test('has a Sign in button', async ({ page }) => {
    await expect(page.getByRole('button', { name: /sign in/i })).toBeVisible();
  });

  test('page container has login-page CSS class', async ({ page }) => {
    await expect(page.locator('.login-page')).toBeVisible();
  });

  test('no error message is shown on initial load', async ({ page }) => {
    await expect(page.locator('p.error')).toHaveCount(0);
  });
});

test.describe('Successful login', () => {
  test.beforeEach(async ({ page }) => {
    if (!(await frontendReachable())) {
      test.skip(true, 'Frontend not reachable');
    }
  });

  test('admin login redirects to home page', async ({ page }) => {
    await loginAs(page, ADMIN_USER, ADMIN_PASS);
    expect(page.url()).toMatch(/\/$/);
  });

  test('topbar shows username after login', async ({ page }) => {
    await loginAs(page, ADMIN_USER, ADMIN_PASS);
    await expect(page.locator('.topbar .user')).toContainText(ADMIN_USER);
  });

  test('topbar shows app name after login', async ({ page }) => {
    await loginAs(page, ADMIN_USER, ADMIN_PASS);
    await expect(page.locator('.topbar h1')).toContainText('Field Service Ops Hub');
  });

  test('topbar shows role label after login', async ({ page }) => {
    await loginAs(page, ADMIN_USER, ADMIN_PASS);
    // Administrator role display name
    await expect(page.locator('.topbar .role')).toContainText('Administrator');
  });
});

test.describe('Invalid credentials', () => {
  test.beforeEach(async ({ page }) => {
    if (!(await frontendReachable())) {
      test.skip(true, 'Frontend not reachable');
    }
    await page.goto('/login');
  });

  test('wrong password shows error message', async ({ page }) => {
    await page.locator('input[type="text"]').first().fill('admin');
    await page.locator('input[type="password"]').first().fill('wrong-password');
    await page.getByRole('button', { name: /sign in/i }).click();
    // p.error (class: "error" in RSX) should appear
    const errorEl = page.locator('p.error');
    await expect(errorEl).toBeVisible({ timeout: 10_000 });
    await expect(errorEl).toContainText(/invalid|wrong|incorrect/i);
  });

  test('unknown user shows error message', async ({ page }) => {
    await page.locator('input[type="text"]').first().fill('no_such_user_xyz');
    await page.locator('input[type="password"]').first().fill('anything');
    await page.getByRole('button', { name: /sign in/i }).click();
    await expect(page.locator('p.error')).toBeVisible({ timeout: 10_000 });
  });

  test('error clears on subsequent successful login', async ({ page }) => {
    // Trigger an error
    await page.locator('input[type="text"]').first().fill('admin');
    await page.locator('input[type="password"]').first().fill('badpass');
    await page.getByRole('button', { name: /sign in/i }).click();
    await expect(page.locator('p.error')).toBeVisible({ timeout: 10_000 });

    // Now log in correctly — should navigate away, so no error visible
    await page.locator('input[type="password"]').first().fill(ADMIN_PASS);
    await page.getByRole('button', { name: /sign in/i }).click();
    await page.waitForURL('/', { timeout: 15_000 });
    await expect(page.locator('p.error')).toHaveCount(0);
  });
});

test.describe('Logout', () => {
  test.beforeEach(async ({ page }) => {
    if (!(await frontendReachable())) {
      test.skip(true, 'Frontend not reachable');
    }
  });

  test('Sign out button navigates to /login', async ({ page }) => {
    await loginAs(page, ADMIN_USER, ADMIN_PASS);
    await logout(page);
    expect(page.url()).toContain('/login');
  });

  test('after logout the topbar and nav are gone', async ({ page }) => {
    await loginAs(page, ADMIN_USER, ADMIN_PASS);
    await logout(page);
    await expect(page.locator('.topbar')).toHaveCount(0);
    await expect(page.locator('.sidebar')).toHaveCount(0);
  });

  test('after logout visiting / redirects to /login or shows unauthenticated message', async ({
    page,
  }) => {
    await loginAs(page, ADMIN_USER, ADMIN_PASS);
    await logout(page);
    await page.goto('/');
    // Either redirected to /login, or shows "not signed in" message
    const url = page.url();
    const body = await page.content();
    const isLoginPage = url.includes('/login');
    const isUnauthMessage =
      body.includes('not signed in') || body.includes('You are not signed in');
    expect(isLoginPage || isUnauthMessage).toBe(true);
  });
});

test.describe('Unauthenticated access', () => {
  test.beforeEach(async ({ page }) => {
    if (!(await frontendReachable())) {
      test.skip(true, 'Frontend not reachable');
    }
    // Ensure no auth in storage before each test
    await page.context().clearCookies();
    await page.goto('/login');
    await page.evaluate(() => localStorage.clear());
  });

  test('/ without auth shows not-signed-in content', async ({ page }) => {
    await page.goto('/');
    const url = page.url();
    const body = await page.content();
    const redirectedToLogin = url.includes('/login');
    const showsUnauthMessage =
      body.includes('not signed in') || body.includes('You are not signed in') ||
      body.includes('Go to sign in');
    expect(redirectedToLogin || showsUnauthMessage).toBe(true);
  });

  test('/catalog without auth shows not-signed-in content', async ({ page }) => {
    await page.goto('/catalog');
    const body = await page.content();
    const url = page.url();
    const redirectedToLogin = url.includes('/login');
    const showsUnauthMessage =
      body.includes('not signed in') || body.includes('You are not signed in') ||
      body.includes('Go to sign in');
    expect(redirectedToLogin || showsUnauthMessage).toBe(true);
  });
});

test.describe('Session persistence', () => {
  test.beforeEach(async ({ page }) => {
    if (!(await frontendReachable())) {
      test.skip(true, 'Frontend not reachable');
    }
  });

  test('reload after login keeps user logged in', async ({ page }) => {
    await loginAs(page, ADMIN_USER, ADMIN_PASS);
    // Reload and verify we're still on the home page, not kicked to /login
    await page.reload();
    await page.waitForLoadState('networkidle');
    // Should still be authenticated
    await expect(page.locator('.topbar .user')).toContainText(ADMIN_USER, { timeout: 10_000 });
  });
});

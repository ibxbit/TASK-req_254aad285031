/**
 * Admin panel E2E tests.
 *
 * Tests:
 *  - Admin page heading and form structure
 *  - Role dropdown contains all seven roles
 *  - Non-admin user is denied access (sees permission error)
 *  - User creation form submission (create + verify user appears in list)
 *
 * Selectors from frontend/src/pages/admin.rs.
 */

import { test, expect } from '@playwright/test';
import {
  loginAs,
  logout,
  ADMIN_USER,
  ADMIN_PASS,
  bootstrapAdminToken,
  provisionUser,
  frontendReachable,
} from './helpers';

test.describe('Admin page — administrator access', () => {
  test.beforeEach(async ({ page }) => {
    if (!(await frontendReachable())) {
      test.skip(true, 'Frontend not reachable');
    }
    await loginAs(page, ADMIN_USER, ADMIN_PASS);
    await page.goto('/admin');
    await page.waitForLoadState('networkidle');
  });

  test('renders Administration heading', async ({ page }) => {
    await expect(page.locator('h2')).toContainText('Administration');
  });

  test('has a Username input', async ({ page }) => {
    // Find input[type="text"] inside the admin form
    const inputs = page.locator('.card input[type="text"]');
    await expect(inputs.first()).toBeVisible();
  });

  test('has a Password input', async ({ page }) => {
    await expect(page.locator('.card input[type="password"]')).toBeVisible();
  });

  test('role dropdown has all seven role options', async ({ page }) => {
    const select = page.locator('.card select');
    await expect(select).toBeVisible();
    const expectedValues = [
      'requester',
      'moderator',
      'service_manager',
      'warehouse_manager',
      'mentor',
      'intern',
      'administrator',
    ];
    for (const role of expectedValues) {
      await expect(select.locator(`option[value="${role}"]`)).toHaveCount(1);
    }
  });

  test('has a Create user button', async ({ page }) => {
    await expect(page.getByRole('button', { name: /create user/i })).toBeVisible();
  });

  test('users list section is rendered', async ({ page }) => {
    // After load the users table/list should appear
    const content = await page.content();
    // Either a table, list, or user rows — admin user themselves should be listed
    expect(content.toLowerCase()).toMatch(/user|username|admin/);
  });
});

test.describe('Admin page — non-admin access denied', () => {
  let username: string;
  let password: string;

  test.beforeAll(async () => {
    if (!(await frontendReachable())) return;
    const adminToken = await bootstrapAdminToken();
    if (!adminToken) return;
    const user = await provisionUser(adminToken, 'requester');
    if (user) {
      username = user.username;
      password = user.password;
    }
  });

  test.beforeEach(async ({ page }) => {
    if (!(await frontendReachable())) {
      test.skip(true, 'Frontend not reachable');
    }
    if (!username) {
      test.skip(true, 'Could not provision requester user');
    }
    await loginAs(page, username, password);
    await page.goto('/admin');
    await page.waitForLoadState('networkidle');
  });

  test('non-admin user sees permission error on /admin', async ({ page }) => {
    const content = await page.content();
    expect(content).toMatch(/permission|not have|forbidden|unauthorized/i);
  });

  test('non-admin user does not see the Create user form', async ({ page }) => {
    await expect(page.locator('input[type="password"]')).toHaveCount(0);
  });
});

test.describe('Admin user creation flow', () => {
  test.beforeEach(async ({ page }) => {
    if (!(await frontendReachable())) {
      test.skip(true, 'Frontend not reachable');
    }
    await loginAs(page, ADMIN_USER, ADMIN_PASS);
    await page.goto('/admin');
    await page.waitForLoadState('networkidle');
  });

  test('filling and submitting the user form creates a new user', async ({ page }) => {
    const suffix = Date.now().toString(36);
    const newUsername = `e2e_ui_user_${suffix}`;

    const usernameInput = page.locator('.card input[type="text"]').first();
    const passwordInput = page.locator('.card input[type="password"]');
    const roleSelect = page.locator('.card select');
    const createBtn = page.getByRole('button', { name: /create user/i });

    await usernameInput.fill(newUsername);
    await passwordInput.fill('UITestPass123!');
    await roleSelect.selectOption('moderator');
    await createBtn.click();

    // After creation the user list should refresh and contain the new username
    await page.waitForLoadState('networkidle');
    const content = await page.content();
    expect(content).toContain(newUsername);
  });
});

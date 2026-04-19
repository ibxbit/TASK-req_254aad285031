/**
 * Forum page E2E tests.
 *
 * Tests:
 *  - Page heading and board list section
 *  - Create Post form (title input, textarea, Post button)
 *  - Admin-only controls visible for Administrator, hidden for Requester
 *  - Post creation flow
 *
 * Selectors from frontend/src/pages/forum.rs.
 */

import { test, expect } from '@playwright/test';
import {
  loginAs,
  ADMIN_USER,
  ADMIN_PASS,
  bootstrapAdminToken,
  provisionUser,
  frontendReachable,
} from './helpers';

test.describe('Forum page — structure', () => {
  test.beforeEach(async ({ page }) => {
    if (!(await frontendReachable())) {
      test.skip(true, 'Frontend not reachable');
    }
    await loginAs(page, ADMIN_USER, ADMIN_PASS);
    await page.goto('/forum');
    await page.waitForLoadState('networkidle');
  });

  test('renders Forum heading', async ({ page }) => {
    await expect(page.locator('h2')).toContainText('Forum');
  });

  test('has a boards section', async ({ page }) => {
    const content = await page.content();
    expect(content).toContain('Boards');
  });

  test('has a Title input for creating posts', async ({ page }) => {
    // Find input with placeholder "Post title" or just a text input in the post card
    const content = await page.content();
    expect(content).toContain('Title');
    const inputs = page.locator('input[type="text"]');
    // At least one text input for title
    await expect(inputs.first()).toBeVisible();
  });

  test('has a Content textarea for creating posts', async ({ page }) => {
    await expect(page.locator('textarea')).toBeVisible();
  });

  test('has a Post button', async ({ page }) => {
    await expect(page.getByRole('button', { name: /post/i })).toBeVisible();
  });
});

test.describe('Forum page — admin-only controls', () => {
  test.beforeEach(async ({ page }) => {
    if (!(await frontendReachable())) {
      test.skip(true, 'Frontend not reachable');
    }
    await loginAs(page, ADMIN_USER, ADMIN_PASS);
    await page.goto('/forum');
    await page.waitForLoadState('networkidle');
  });

  test('admin sees Create Board section', async ({ page }) => {
    const content = await page.content();
    expect(content).toMatch(/create board|board name|new board/i);
  });

  test('admin sees zone/board management UI', async ({ page }) => {
    const content = await page.content();
    // Admin gets zone management controls
    expect(content).toMatch(/zone|Zone/);
  });
});

test.describe('Forum page — requester access', () => {
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
    await page.goto('/forum');
    await page.waitForLoadState('networkidle');
  });

  test('requester sees Forum page', async ({ page }) => {
    await expect(page.locator('h2')).toContainText('Forum');
  });

  test('requester can see boards section', async ({ page }) => {
    const content = await page.content();
    expect(content).toContain('Boards');
  });

  test('requester does not see admin-only board create form', async ({ page }) => {
    const content = await page.content();
    // Admin-only section header "Create Board" should not appear for requester
    // (the admin guard hides it with `if is_admin { ... }`)
    expect(content).not.toMatch(/create board/i);
  });
});

test.describe('Forum post creation flow', () => {
  test.beforeEach(async ({ page }) => {
    if (!(await frontendReachable())) {
      test.skip(true, 'Frontend not reachable');
    }
    await loginAs(page, ADMIN_USER, ADMIN_PASS);
    await page.goto('/forum');
    await page.waitForLoadState('networkidle');
  });

  test('filling title and content then clicking Post does not crash the page', async ({ page }) => {
    // Fill the post form — whether it succeeds depends on a board being selected,
    // but we're testing the UI interaction, not the API call outcome.
    const titleInput = page.locator('input[type="text"]').last();
    const contentArea = page.locator('textarea').last();

    await titleInput.fill('E2E test post title');
    await contentArea.fill('E2E test post content');

    await page.getByRole('button', { name: /^post$/i }).click();
    await page.waitForLoadState('networkidle');

    // Page should still be on /forum — no crash
    expect(page.url()).toContain('/forum');
  });
});

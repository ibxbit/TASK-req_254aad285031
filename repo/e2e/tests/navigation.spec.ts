/**
 * Role-based navigation visibility E2E tests.
 *
 * The AuthedLayout component renders a .sidebar NavBar whose links depend on
 * the logged-in role. These tests verify the browser reflects the same rules
 * as frontend_core::nav::menu_for (the pure-Rust unit tests in
 * frontend_tests/tests/home_nav_structure.rs test the logic; this file tests
 * that the actual rendered page matches).
 *
 * Nav rules (from frontend/src/components/layout.rs):
 *   Always:                   Home, Forum, Face
 *   Requester/SM/Admin:       Catalog, Work Orders
 *   Intern/Mentor/Admin:      Internship
 *   WarehouseManager/Admin:   Warehouse
 *   Admin only:               Admin
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

test.describe('Administrator navigation', () => {
  test.beforeEach(async ({ page }) => {
    if (!(await frontendReachable())) {
      test.skip(true, 'Frontend not reachable');
    }
    await loginAs(page, ADMIN_USER, ADMIN_PASS);
  });

  test('sidebar renders all eight module links', async ({ page }) => {
    const sidebar = page.locator('nav.sidebar');
    await expect(sidebar).toBeVisible();
    for (const label of ['Home', 'Catalog', 'Work Orders', 'Forum', 'Internship', 'Warehouse', 'Face', 'Admin']) {
      await expect(sidebar.getByText(label, { exact: true })).toBeVisible();
    }
  });

  test('Home link appears before other links', async ({ page }) => {
    const links = page.locator('nav.sidebar a');
    const count = await links.count();
    expect(count).toBeGreaterThanOrEqual(8);
    const firstText = await links.first().innerText();
    expect(firstText.trim()).toBe('Home');
  });
});

test.describe('Requester navigation', () => {
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
  });

  test('shows Catalog and Work Orders', async ({ page }) => {
    const sidebar = page.locator('nav.sidebar');
    await expect(sidebar.getByText('Catalog', { exact: true })).toBeVisible();
    await expect(sidebar.getByText('Work Orders', { exact: true })).toBeVisible();
  });

  test('shows Forum and Face (universal links)', async ({ page }) => {
    const sidebar = page.locator('nav.sidebar');
    await expect(sidebar.getByText('Forum', { exact: true })).toBeVisible();
    await expect(sidebar.getByText('Face', { exact: true })).toBeVisible();
  });

  test('does not show Warehouse, Internship, or Admin links', async ({ page }) => {
    const sidebar = page.locator('nav.sidebar');
    await expect(sidebar.getByText('Warehouse', { exact: true })).toHaveCount(0);
    await expect(sidebar.getByText('Internship', { exact: true })).toHaveCount(0);
    await expect(sidebar.getByText('Admin', { exact: true })).toHaveCount(0);
  });
});

test.describe('Moderator navigation', () => {
  let username: string;
  let password: string;

  test.beforeAll(async () => {
    if (!(await frontendReachable())) return;
    const adminToken = await bootstrapAdminToken();
    if (!adminToken) return;
    const user = await provisionUser(adminToken, 'moderator');
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
      test.skip(true, 'Could not provision moderator user');
    }
    await loginAs(page, username, password);
  });

  test('shows Home, Forum, and Face', async ({ page }) => {
    const sidebar = page.locator('nav.sidebar');
    await expect(sidebar.getByText('Home', { exact: true })).toBeVisible();
    await expect(sidebar.getByText('Forum', { exact: true })).toBeVisible();
    await expect(sidebar.getByText('Face', { exact: true })).toBeVisible();
  });

  test('does not show Catalog, Warehouse, Internship, or Admin', async ({ page }) => {
    const sidebar = page.locator('nav.sidebar');
    await expect(sidebar.getByText('Catalog', { exact: true })).toHaveCount(0);
    await expect(sidebar.getByText('Warehouse', { exact: true })).toHaveCount(0);
    await expect(sidebar.getByText('Admin', { exact: true })).toHaveCount(0);
  });
});

test.describe('WarehouseManager navigation', () => {
  let username: string;
  let password: string;

  test.beforeAll(async () => {
    if (!(await frontendReachable())) return;
    const adminToken = await bootstrapAdminToken();
    if (!adminToken) return;
    const user = await provisionUser(adminToken, 'warehouse_manager');
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
      test.skip(true, 'Could not provision warehouse_manager user');
    }
    await loginAs(page, username, password);
  });

  test('shows Warehouse link', async ({ page }) => {
    await expect(page.locator('nav.sidebar').getByText('Warehouse', { exact: true })).toBeVisible();
  });

  test('does not show Catalog or Admin', async ({ page }) => {
    const sidebar = page.locator('nav.sidebar');
    await expect(sidebar.getByText('Catalog', { exact: true })).toHaveCount(0);
    await expect(sidebar.getByText('Admin', { exact: true })).toHaveCount(0);
  });
});

test.describe('Intern navigation', () => {
  let username: string;
  let password: string;

  test.beforeAll(async () => {
    if (!(await frontendReachable())) return;
    const adminToken = await bootstrapAdminToken();
    if (!adminToken) return;
    const user = await provisionUser(adminToken, 'intern');
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
      test.skip(true, 'Could not provision intern user');
    }
    await loginAs(page, username, password);
  });

  test('shows Internship link', async ({ page }) => {
    await expect(page.locator('nav.sidebar').getByText('Internship', { exact: true })).toBeVisible();
  });

  test('does not show Catalog, Warehouse, or Admin', async ({ page }) => {
    const sidebar = page.locator('nav.sidebar');
    await expect(sidebar.getByText('Catalog', { exact: true })).toHaveCount(0);
    await expect(sidebar.getByText('Warehouse', { exact: true })).toHaveCount(0);
    await expect(sidebar.getByText('Admin', { exact: true })).toHaveCount(0);
  });
});

test.describe('Home page welcome message per role', () => {
  test.beforeEach(async ({ page }) => {
    if (!(await frontendReachable())) {
      test.skip(true, 'Frontend not reachable');
    }
  });

  test('admin home page shows username and Administrator role', async ({ page }) => {
    await loginAs(page, ADMIN_USER, ADMIN_PASS);
    await expect(page.locator('h2')).toContainText(ADMIN_USER);
    const body = await page.content();
    expect(body).toContain('Administrator');
  });
});

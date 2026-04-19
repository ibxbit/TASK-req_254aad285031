/**
 * Warehouse page E2E tests.
 *
 * Selectors anchored to frontend/src/pages/warehouse.rs.
 *
 * Page structure:
 *   h2 "Warehouse"
 *   card "Tree": always visible to all authenticated users; shows tree or
 *     "No warehouses." when empty
 *   (if !can_mutate) p.muted "You need the warehouse_manager or administrator
 *     role to mutate structure."
 *   (if can_mutate) card "Warehouses": "New name" input, Create warehouse button
 *   (if can_mutate) card "Zones": Zone creation inputs
 *   (if can_mutate) card "Bins": Zone id / Name / Temp zone inputs, Create bin button
 *   card "Change history": visible to can_mutate users
 *
 * Role: Administrator and WarehouseManager have `can_mutate = true`.
 * Requester can navigate to /warehouse but sees only the Tree + a role warning.
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

test.describe('Warehouse page — administrator access', () => {
  test.beforeEach(async ({ page }) => {
    if (!(await frontendReachable())) {
      test.skip(true, 'Frontend not reachable');
    }
    await loginAs(page, ADMIN_USER, ADMIN_PASS);
    await page.goto('/warehouse');
    await page.waitForLoadState('networkidle');
  });

  test('renders Warehouse heading', async ({ page }) => {
    await expect(page.locator('h2')).toContainText('Warehouse');
  });

  test('has a Tree card section (read-only tree view)', async ({ page }) => {
    const content = await page.content();
    expect(content).toContain('Tree');
  });

  test('has Warehouses mutation card (admin has can_mutate)', async ({ page }) => {
    const content = await page.content();
    expect(content).toContain('Warehouses');
  });

  test('has Create warehouse button', async ({ page }) => {
    await expect(page.getByRole('button', { name: /create warehouse/i })).toBeVisible();
  });

  test('has Zones card', async ({ page }) => {
    const content = await page.content();
    expect(content).toContain('Zones');
  });

  test('has Bins card with Create bin button', async ({ page }) => {
    const content = await page.content();
    expect(content).toContain('Bins');
    await expect(page.getByRole('button', { name: /create bin/i })).toBeVisible();
  });

  test('has Change history card', async ({ page }) => {
    const content = await page.content();
    expect(content).toContain('Change history');
  });

  test('bin creation inputs are present (Zone id, Name, Temp zone)', async ({ page }) => {
    const content = await page.content();
    expect(content).toContain('Zone id');
    expect(content).toContain('Temp zone');
  });
});

test.describe('Warehouse page — requester sees read-only tree', () => {
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

  test('Warehouse link is absent from requester sidebar', async ({ page }) => {
    await expect(page.locator('nav.sidebar').getByText('Warehouse', { exact: true })).toHaveCount(0);
  });

  test('navigating to /warehouse shows the page with a role warning', async ({ page }) => {
    await page.goto('/warehouse');
    await page.waitForLoadState('networkidle');
    const content = await page.content();
    // Page renders for authenticated users but shows a role warning
    // and hides mutation controls.
    expect(content).toContain('Warehouse');
    // Warning message for non-can_mutate users
    expect(content).toMatch(/warehouse_manager|administrator role|mutate/i);
  });

  test('requester on /warehouse does not see Create warehouse button', async ({ page }) => {
    await page.goto('/warehouse');
    await page.waitForLoadState('networkidle');
    await expect(page.getByRole('button', { name: /create warehouse/i })).toHaveCount(0);
  });
});

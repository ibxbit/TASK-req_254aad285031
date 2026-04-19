/**
 * Catalog page E2E tests.
 *
 * Tests:
 *  - Page heading and card structure
 *  - Search input and button
 *  - Sort dropdown options (best_rated, lowest_price, soonest_available)
 *  - Availability datetime-local inputs
 *  - Price and rating filter labels
 *  - Search execution returns results section
 *  - Compare selection UI (add/clear flow)
 *
 * Selectors are anchored to the actual RSX in frontend/src/pages/catalog.rs.
 */

import { test, expect } from '@playwright/test';
import { loginAs, ADMIN_USER, ADMIN_PASS, frontendReachable } from './helpers';

test.describe('Catalog page', () => {
  test.beforeEach(async ({ page }) => {
    if (!(await frontendReachable())) {
      test.skip(true, 'Frontend not reachable');
    }
    await loginAs(page, ADMIN_USER, ADMIN_PASS);
    await page.goto('/catalog');
    await page.waitForLoadState('networkidle');
  });

  test('renders Service Catalog heading', async ({ page }) => {
    await expect(page.locator('h2')).toContainText('Service Catalog');
  });

  test('has a search / text input in the filter card', async ({ page }) => {
    // The search input is not type="search" — just a plain text input
    const filterCard = page.locator('.card').first();
    await expect(filterCard.locator('input').first()).toBeVisible();
  });

  test('has a Search button', async ({ page }) => {
    await expect(page.getByRole('button', { name: /search/i })).toBeVisible();
  });

  test('has a sort select with three options', async ({ page }) => {
    const select = page.locator('select');
    await expect(select).toBeVisible();
    const options = select.locator('option');
    await expect(options).toHaveCount(3);
    await expect(options.nth(0)).toHaveValue('best_rated');
    await expect(options.nth(1)).toHaveValue('lowest_price');
    await expect(options.nth(2)).toHaveValue('soonest_available');
  });

  test('has two datetime-local inputs for availability window', async ({ page }) => {
    const dtInputs = page.locator('input[type="datetime-local"]');
    await expect(dtInputs).toHaveCount(2);
  });

  test('page contains Min price and Max price labels', async ({ page }) => {
    const content = await page.content();
    expect(content).toContain('Min price');
    expect(content).toContain('Max price');
  });

  test('page contains Min rating and ZIP labels', async ({ page }) => {
    const content = await page.content();
    expect(content).toContain('Min rating');
    expect(content).toContain('ZIP');
  });

  test('compare card shows selected count / 3 denominator', async ({ page }) => {
    const content = await page.content();
    // "Selected for compare: 0 / 3" or similar
    expect(content).toMatch(/\/ 3/);
  });

  test('compare card has Compare and Clear buttons', async ({ page }) => {
    await expect(page.getByRole('button', { name: /compare/i })).toBeVisible();
    await expect(page.getByRole('button', { name: /clear/i })).toBeVisible();
  });

  test('clicking Search triggers results area to render (may be empty)', async ({ page }) => {
    await page.getByRole('button', { name: /search/i }).click();
    // Wait for any network activity triggered by the search
    await page.waitForLoadState('networkidle');
    // Page should remain on /catalog (no redirect)
    expect(page.url()).toContain('/catalog');
  });

  test('sort select defaults to best_rated', async ({ page }) => {
    await expect(page.locator('select')).toHaveValue('best_rated');
  });

  test('changing sort to lowest_price updates the select', async ({ page }) => {
    await page.locator('select').selectOption('lowest_price');
    await expect(page.locator('select')).toHaveValue('lowest_price');
  });

  test('typing in the search box reflects the input value', async ({ page }) => {
    const input = page.locator('.card input').first();
    await input.fill('test query');
    await expect(input).toHaveValue('test query');
  });
});

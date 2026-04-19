/**
 * Shared utilities for Field Service Hub E2E tests.
 *
 * All UI interaction is anchored to real DOM selectors derived from the
 * actual Dioxus RSX in frontend/src/. No test IDs are required.
 */

import { Page } from '@playwright/test';

export const FRONTEND_URL = process.env.FRONTEND_URL ?? 'http://127.0.0.1:3000';
export const API_URL = process.env.API_BASE ?? 'http://127.0.0.1:8000';

export const ADMIN_USER = process.env.ADMIN_USER ?? 'admin';
export const ADMIN_PASS = process.env.ADMIN_PASS ?? 'change-me-please-now';

// ---- UI helpers ----

/**
 * Navigate to /login and submit credentials.
 * Waits for the redirect to "/" before returning.
 * The login page uses onclick (not type="submit"), so we click the button.
 */
export async function loginAs(page: Page, username: string, password: string): Promise<void> {
  await page.goto('/login');
  // Input elements have no `name` attribute in Dioxus RSX; select by type.
  await page.locator('input[type="text"]').first().fill(username);
  await page.locator('input[type="password"]').first().fill(password);
  await page.getByRole('button', { name: /sign in/i }).click();
  await page.waitForURL('/', { timeout: 20_000 });
}

/**
 * Click the "Sign out" button in the topbar (.signout) and wait for /login.
 */
export async function logout(page: Page): Promise<void> {
  await page.locator('button.signout').click();
  await page.waitForURL('/login', { timeout: 10_000 });
}

// ---- API helpers (fetch against the backend directly) ----

/**
 * Obtain a bearer token from the backend. Returns null if the backend
 * is unreachable or credentials are wrong — callers should skip gracefully.
 */
export async function apiLogin(username: string, password: string): Promise<string | null> {
  try {
    const resp = await fetch(`${API_URL}/api/auth/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username, password }),
    });
    if (!resp.ok) return null;
    const data = await resp.json() as { token?: string };
    return data.token ?? null;
  } catch {
    return null;
  }
}

/**
 * Bootstrap an admin token using the well-known quickstart credentials.
 * Falls back to POST /auth/register if the table is still empty.
 */
export async function bootstrapAdminToken(): Promise<string | null> {
  const envToken = process.env.API_ADMIN_TOKEN;
  if (envToken) return envToken;

  const envUser = process.env.ADMIN_USER ?? 'admin';
  const envPass = process.env.ADMIN_PASS ?? 'change-me-please-now';
  const direct = await apiLogin(envUser, envPass);
  if (direct) return direct;

  // Attempt first-run registration (succeeds only when users table is empty).
  try {
    await fetch(`${API_URL}/api/auth/register`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username: 'admin', password: 'change-me-please-now' }),
    });
  } catch {
    // ignore
  }
  return apiLogin('admin', 'change-me-please-now');
}

/**
 * Provision a user via the admin API. Returns { username, password } or null.
 */
export async function provisionUser(
  adminToken: string,
  role: string,
): Promise<{ username: string; password: string } | null> {
  const suffix = Date.now().toString(36);
  const username = `e2e_${role}_${suffix}`;
  const password = 'e2ePass123!';
  try {
    const resp = await fetch(`${API_URL}/api/admin/users`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${adminToken}`,
      },
      body: JSON.stringify({ username, password, role }),
    });
    if (!resp.ok) return null;
    return { username, password };
  } catch {
    return null;
  }
}

/**
 * Check whether the frontend is reachable. If not, skip the test with a
 * clear message rather than a confusing timeout failure.
 */
export async function frontendReachable(): Promise<boolean> {
  try {
    const resp = await fetch(FRONTEND_URL, { signal: AbortSignal.timeout(3_000) });
    return resp.ok || resp.status < 500;
  } catch {
    return false;
  }
}

/**
 * Check whether the backend API is reachable.
 */
export async function backendReachable(): Promise<boolean> {
  try {
    const resp = await fetch(`${API_URL}/api/health`, { signal: AbortSignal.timeout(3_000) });
    return resp.ok;
  } catch {
    return false;
  }
}

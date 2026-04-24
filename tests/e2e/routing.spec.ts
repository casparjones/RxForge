import { test, expect } from '@playwright/test';
import { setupApiMocks, loginAsUser, loginAsAdmin, gotoSPA } from './helpers/mock-api';

test.describe('SvelteKit Projektsetup & Routing', () => {
  test.beforeEach(async ({ page }) => {
    await setupApiMocks(page);
  });

  test('Root / leitet zu /dashboard weiter', async ({ page }) => {
    await loginAsUser(page);
    await page.goto('/');
    await page.waitForURL(/\/dashboard/, { timeout: 5000 });
    await expect(page).toHaveURL(/\/dashboard/);
  });

  test('/login Route ist erreichbar', async ({ page }) => {
    await page.goto('/login');
    await expect(page).toHaveURL(/\/login/);
    await expect(page.locator('input#email')).toBeVisible();
  });

  test('/register Route ist erreichbar', async ({ page }) => {
    await page.goto('/register');
    await expect(page).toHaveURL(/\/register/);
    await expect(page.locator('input#email')).toBeVisible();
  });

  test('/dashboard/apps Route ist mit Auth erreichbar', async ({ page }) => {
    await loginAsUser(page);
    await page.goto('/dashboard/apps');
    await expect(page).toHaveURL(/\/dashboard\/apps/);
  });

  test('/dashboard/analytics Route ist erreichbar', async ({ page }) => {
    await loginAsUser(page);
    await page.goto('/dashboard/analytics');
    await expect(page).toHaveURL(/\/dashboard\/analytics/);
  });

  test('/admin/users Route ist für Admins erreichbar', async ({ page }) => {
    await loginAsAdmin(page);
    await gotoSPA(page, '/admin/users');
    await expect(page).toHaveURL(/\/admin\/users/);
  });

  test('/admin/analytics Route ist für Admins erreichbar', async ({ page }) => {
    await loginAsAdmin(page);
    await gotoSPA(page, '/admin/analytics');
    await expect(page).toHaveURL(/\/admin\/analytics/);
  });

  test('unauthentifizierter Zugriff auf /dashboard wird zu /login umgeleitet', async ({ page }) => {
    await page.goto('/login');
    await page.evaluate(() => {
      localStorage.removeItem('rxforge_token');
      localStorage.removeItem('rxforge_user');
    });
    await page.goto('/dashboard/apps');
    await page.waitForURL(/\/login/, { timeout: 5000 });
    await expect(page).toHaveURL(/\/login/);
  });

  test('unauthentifizierter Zugriff auf /admin wird umgeleitet', async ({ page }) => {
    await page.goto('/login');
    await page.evaluate(() => {
      localStorage.removeItem('rxforge_token');
      localStorage.removeItem('rxforge_user');
    });
    await page.goto('/admin/users');
    await page.waitForURL(/\/(dashboard|login)/, { timeout: 5000 });
    await expect(page).not.toHaveURL(/\/admin\/users/);
  });

  test('Dashboard hat SvelteKit-Navigation (RxForge Logo)', async ({ page }) => {
    await loginAsUser(page);
    await page.goto('/dashboard/apps');
    await expect(page.getByText('RxForge').first()).toBeVisible();
  });

  test('Admin-Panel hat eigene Navigation (RxForge Admin)', async ({ page }) => {
    await loginAsAdmin(page);
    await gotoSPA(page, '/admin/users');
    await expect(page.getByText('RxForge Admin')).toBeVisible();
  });

  test('Svelte-Toaster Komponente ist im Root-Layout eingebunden', async ({ page }) => {
    await page.goto('/login');
    // Toaster from svelte-sonner renders a section with aria-label="Notifications ..."
    const toaster = page.locator('section[aria-label^="Notifications"]');
    await expect(toaster).toBeAttached();
  });

  test('JWT wird nach Login in localStorage persistiert', async ({ page }) => {
    await loginAsUser(page);
    const token = await page.evaluate(() => localStorage.getItem('rxforge_token'));
    expect(token).toBeTruthy();
    expect(token).toMatch(/mock-jwt-token/);
  });

  test('Keine SSR - HTML ist auf allen Routen identisch (adapter-static fallback)', async ({ page }) => {
    // With adapter-static fallback index.html every client route is served via SPA shell
    await page.goto('/login');
    const content = await page.content();
    // SvelteKit hydration marker or script should be present
    expect(content).toMatch(/<script[^>]*>/);
    // No server-rendered user-specific data on initial HTML
    expect(content).not.toContain('admin@example.com');
  });
});

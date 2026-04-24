import { test, expect } from '@playwright/test';
import { setupApiMocks, loginAsAdmin, loginAsUser, gotoSPA } from './helpers/mock-api';

test.describe('Analytics Dashboard – Charts & Statistiken', () => {
  test.beforeEach(async ({ page }) => {
    await setupApiMocks(page);
  });

  test('Dashboard Analytics-Seite ist erreichbar', async ({ page }) => {
    await loginAsUser(page);
    await page.goto('/dashboard/analytics');
    await expect(page).toHaveURL(/\/dashboard\/analytics/);
    await expect(page.getByRole('heading', { name: /app analytics/i })).toBeVisible({ timeout: 5000 });
  });

  test('Analytics-Seite zeigt Stat-Cards (Today, 7 Days, 30 Days)', async ({ page }) => {
    await loginAsUser(page);
    await page.goto('/dashboard/analytics');
    await expect(page.getByText(/requests today/i)).toBeVisible({ timeout: 10000 });
    await expect(page.getByText('Last 7 Days', { exact: true })).toBeVisible();
    await expect(page.getByText('Last 30 Days', { exact: true })).toBeVisible();
  });

  test('Analytics-Seite zeigt Zahlen aus API-Mock', async ({ page }) => {
    await loginAsUser(page);
    await page.goto('/dashboard/analytics');
    // Wait for stats to load (requests_today: 42)
    await expect(page.getByText('42')).toBeVisible({ timeout: 10000 });
    await expect(page.getByText('300')).toBeVisible();
    await expect(page.getByText('1200')).toBeVisible();
  });

  test('Chart-Canvas ist sichtbar (svelte-chartjs Line chart)', async ({ page }) => {
    await loginAsUser(page);
    await page.goto('/dashboard/analytics');
    // Wait for data to load first
    await expect(page.getByText('42')).toBeVisible({ timeout: 10000 });
    // Canvas from svelte-chartjs Line
    await expect(page.locator('canvas').first()).toBeVisible({ timeout: 5000 });
  });

  test('Analytics-Seite ohne Auth leitet zu /login', async ({ page }) => {
    await page.goto('/login');
    await page.evaluate(() => {
      localStorage.removeItem('rxforge_token');
      localStorage.removeItem('rxforge_user');
    });
    await page.goto('/dashboard/analytics');
    await page.waitForURL(/\/login/, { timeout: 5000 });
    await expect(page).toHaveURL(/\/login/);
  });

  test('Admin Global Analytics-Seite zeigt Global Analytics Überschrift', async ({ page }) => {
    await loginAsAdmin(page);
    await gotoSPA(page, '/admin/analytics');
    await expect(page.getByRole('heading', { name: /global analytics/i })).toBeVisible({ timeout: 10000 });
  });

  test('Admin Analytics zeigt Total Requests aus Mock-Daten', async ({ page }) => {
    await loginAsAdmin(page);
    await gotoSPA(page, '/admin/analytics');
    await expect(page.getByText('1234')).toBeVisible({ timeout: 10000 });
  });

  test('Admin Analytics zeigt Charts (Canvas-Elemente)', async ({ page }) => {
    await loginAsAdmin(page);
    await gotoSPA(page, '/admin/analytics');
    // Wait for data
    await expect(page.getByText('1234')).toBeVisible({ timeout: 10000 });
    const canvases = page.locator('canvas');
    await expect(canvases.first()).toBeVisible({ timeout: 5000 });
  });

  test('Admin Analytics zeigt Stat-Cards (Total Requests, Active Apps, Active Users)', async ({ page }) => {
    await loginAsAdmin(page);
    await gotoSPA(page, '/admin/analytics');
    await expect(page.getByText('Total Requests', { exact: true })).toBeVisible({ timeout: 10000 });
    await expect(page.getByText('Active Apps', { exact: true })).toBeVisible();
    await expect(page.getByText('Active Users', { exact: true })).toBeVisible();
  });

  test('Dashboard Analytics hat App-Selector für Besitzer mehrerer Apps', async ({ page }) => {
    await loginAsUser(page);
    await page.goto('/dashboard/analytics');
    await expect(page.getByText('42')).toBeVisible({ timeout: 10000 });
    // The select dropdown contains the app option
    const select = page.locator('select');
    await expect(select.first()).toBeVisible();
  });
});

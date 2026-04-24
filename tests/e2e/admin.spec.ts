import { test, expect } from '@playwright/test';
import { setupApiMocks, loginAsAdmin, loginAsUser, loginAsSuperadmin, gotoSPA } from './helpers/mock-api';

test.describe('Admin Panel – Userverwaltung', () => {
  test.beforeEach(async ({ page }) => {
    await setupApiMocks(page);
  });

  test('Admin-Route nicht zugänglich für unauthentifizierte User', async ({ page }) => {
    await page.goto('/login');
    await page.evaluate(() => {
      localStorage.removeItem('rxforge_token');
      localStorage.removeItem('rxforge_user');
    });
    await page.goto('/admin/users');
    await page.waitForURL(/\/(dashboard|login)/, { timeout: 5000 });
    await expect(page).not.toHaveURL(/\/admin\/users/);
  });

  test('Admin-Route nicht zugänglich für reguläre User (role: user)', async ({ page }) => {
    await loginAsUser(page);
    await page.goto('/admin/users');
    await page.waitForURL(/\/(dashboard|login)/, { timeout: 5000 });
    await expect(page).not.toHaveURL(/\/admin\/users/);
  });

  test('User-Liste wird für Admins angezeigt', async ({ page }) => {
    await loginAsAdmin(page);
    await gotoSPA(page, '/admin/users');
    const table = page.getByRole('table');
    await expect(table.getByText('admin@example.com')).toBeVisible({ timeout: 5000 });
    await expect(table.getByText('user@example.com')).toBeVisible();
    await expect(table.getByText('super@example.com')).toBeVisible();
  });

  test('Admin-Panel hat User Management Überschrift', async ({ page }) => {
    await loginAsAdmin(page);
    await gotoSPA(page, '/admin/users');
    await expect(page.getByRole('heading', { name: /user management/i })).toBeVisible();
  });

  test('Suche filtert User-Liste', async ({ page }) => {
    await loginAsAdmin(page);
    await gotoSPA(page, '/admin/users');
    // Wait for user list to render (scope to table to avoid nav-bar email)
    const table = page.getByRole('table');
    await expect(table.getByText('user@example.com')).toBeVisible({ timeout: 5000 });
    const searchInput = page.locator('input[type="search"]');
    await searchInput.fill('super');
    await expect(table.getByText('user@example.com')).not.toBeVisible();
    await expect(table.getByText('super@example.com')).toBeVisible();
  });

  test('Rollen-Filter schränkt User-Liste ein', async ({ page }) => {
    await loginAsAdmin(page);
    await gotoSPA(page, '/admin/users');
    const table = page.getByRole('table');
    await expect(table.getByText('user@example.com')).toBeVisible({ timeout: 5000 });
    // Select "user" in role filter — only user@example.com should remain
    await page.locator('select').first().selectOption('user');
    await expect(table.getByText('user@example.com')).toBeVisible();
    await expect(table.getByText('super@example.com')).not.toBeVisible();
  });

  test('Admin-Navigation zeigt Users und Analytics Links', async ({ page }) => {
    await loginAsAdmin(page);
    await gotoSPA(page, '/admin/users');
    await expect(page.getByRole('link', { name: /^users$/i })).toBeVisible();
    await expect(page.getByRole('link', { name: /^analytics$/i })).toBeVisible();
  });

  test('Manage-Button öffnet User-Slide-over', async ({ page }) => {
    await loginAsAdmin(page);
    await gotoSPA(page, '/admin/users');
    await expect(page.getByRole('table').getByText('admin@example.com')).toBeVisible({ timeout: 5000 });
    await page.getByRole('button', { name: /manage/i }).first().click();
    await expect(page.getByRole('heading', { name: /manage user/i })).toBeVisible();
  });

  test('Slide-over zeigt Rollen-Editor (Select + Save)', async ({ page }) => {
    await loginAsAdmin(page);
    await gotoSPA(page, '/admin/users');
    await expect(page.getByRole('table').getByText('admin@example.com')).toBeVisible({ timeout: 5000 });
    await page.getByRole('button', { name: /manage/i }).first().click();
    await expect(page.locator('select#editRole')).toBeVisible();
    // Save button near role select
    await expect(page.getByRole('button', { name: /^save$/i })).toBeVisible();
  });

  test('Slide-over zeigt Permission-Checkboxen', async ({ page }) => {
    await loginAsAdmin(page);
    await gotoSPA(page, '/admin/users');
    await expect(page.getByRole('table').getByText('admin@example.com')).toBeVisible({ timeout: 5000 });
    await page.getByRole('button', { name: /manage/i }).first().click();
    await expect(page.getByText(/permissions/i).first()).toBeVisible();
    await expect(page.getByText('manage_users')).toBeVisible();
    await expect(page.getByText('manage_apps')).toBeVisible();
    await expect(page.getByText('view_analytics')).toBeVisible();
    await expect(page.getByRole('button', { name: /save permissions/i })).toBeVisible();
  });

  test('Permission-Update sendet PUT an API', async ({ page }) => {
    let captured: any = null;
    await page.route('**/api/v1/admin/users/*/permissions', route => {
      if (route.request().method() === 'PUT') {
        captured = JSON.parse(route.request().postData() || '{}');
      }
      route.fulfill({ json: { ok: true } });
    });
    await loginAsAdmin(page);
    await gotoSPA(page, '/admin/users');
    await expect(page.getByRole('table').getByText('admin@example.com')).toBeVisible({ timeout: 5000 });
    await page.getByRole('button', { name: /manage/i }).first().click();
    // Toggle a permission checkbox (manage_users)
    await page.getByRole('checkbox').first().click();
    await page.getByRole('button', { name: /save permissions/i }).click();
    await expect.poll(() => captured, { timeout: 5000 }).not.toBeNull();
    expect(Array.isArray(captured.permissions)).toBe(true);
  });

  test('Account-Lock Button erscheint und öffnet Confirm-Dialog', async ({ page }) => {
    await loginAsAdmin(page);
    await gotoSPA(page, '/admin/users');
    await expect(page.getByRole('table').getByText('admin@example.com')).toBeVisible({ timeout: 5000 });
    await page.getByRole('button', { name: /manage/i }).first().click();
    // Account Status section
    await expect(page.getByText(/account status/i)).toBeVisible();
    await page.getByRole('button', { name: /^lock$/i }).click();
    // Confirm dialog should pop
    await expect(page.getByRole('heading', { name: /lock account/i })).toBeVisible();
  });

  test('Rollen-Update sendet PUT an API', async ({ page }) => {
    let captured: any = null;
    await page.route('**/api/v1/admin/users/*/role', route => {
      if (route.request().method() === 'PUT') {
        captured = JSON.parse(route.request().postData() || '{}');
      }
      route.fulfill({ json: { ok: true } });
    });
    await loginAsAdmin(page);
    await gotoSPA(page, '/admin/users');
    await expect(page.getByRole('table').getByText('admin@example.com')).toBeVisible({ timeout: 5000 });
    await page.getByRole('button', { name: /manage/i }).first().click();
    await page.locator('select#editRole').selectOption('superadmin');
    await page.getByRole('button', { name: /^save$/i }).click();
    await expect.poll(() => captured, { timeout: 5000 }).not.toBeNull();
    expect(captured.role).toBe('superadmin');
  });

  test('Admin-Route ist für Superadmins zugänglich', async ({ page }) => {
    await loginAsSuperadmin(page);
    await gotoSPA(page, '/admin/users');
    await expect(page).toHaveURL(/\/admin\/users/);
    await expect(page.getByRole('heading', { name: /user management/i })).toBeVisible({ timeout: 5000 });
  });
});

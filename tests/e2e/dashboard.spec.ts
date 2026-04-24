import { test, expect } from '@playwright/test';
import { setupApiMocks, loginAsUser } from './helpers/mock-api';

test.describe('User Dashboard – App-Verwaltung', () => {
  test.beforeEach(async ({ page }) => {
    await setupApiMocks(page);
    await loginAsUser(page);
  });

  test('App-Liste wird angezeigt', async ({ page }) => {
    await page.goto('/dashboard/apps');
    await expect(page.getByText('My App').first()).toBeVisible();
  });

  test('Neue App Button ist sichtbar', async ({ page }) => {
    await page.goto('/dashboard/apps');
    await expect(page.getByRole('button', { name: /\+ new app/i })).toBeVisible();
  });

  test('Create App Modal öffnet sich bei Klick auf New App', async ({ page }) => {
    await page.goto('/dashboard/apps');
    await page.getByRole('button', { name: /\+ new app/i }).click();
    await expect(page.locator('[role="dialog"]')).toBeVisible();
    await expect(page.getByRole('heading', { name: /create new app/i })).toBeVisible();
    await expect(page.locator('input#appName')).toBeVisible();
  });

  test('Create App Modal hat Name und Redirect URIs Felder', async ({ page }) => {
    await page.goto('/dashboard/apps');
    await page.getByRole('button', { name: /\+ new app/i }).click();
    await expect(page.locator('input#appName')).toBeVisible();
    await expect(page.locator('textarea#redirectUris')).toBeVisible();
  });

  test('App erstellen sendet POST an API', async ({ page }) => {
    let captured: any = null;
    await page.route('**/api/v1/apps', route => {
      if (route.request().method() === 'POST') {
        captured = JSON.parse(route.request().postData() || '{}');
        route.fulfill({ json: { id: 'app2', name: captured.name, client_id: 'cid_new', client_secret: 'secret_new', redirect_uris: captured.redirect_uris } });
      } else {
        route.fulfill({ json: [{ id: 'app1', name: 'My App', client_id: 'cid_123', redirect_uris: [] }] });
      }
    });
    await page.goto('/dashboard/apps');
    // Wait for apps list to render (ensures Svelte hydration)
    await expect(page.getByText('My App').first()).toBeVisible();
    await page.getByRole('button', { name: /\+ new app/i }).click();
    await expect(page.locator('[role="dialog"]')).toBeVisible();
    await page.locator('input#appName').fill('Cool New App');
    await page.locator('textarea#redirectUris').fill('https://example.com/cb\nhttps://example.com/cb2');
    await page.getByRole('button', { name: /^create app$/i }).click();
    // Wait for modal to close
    await expect(page.locator('[role="dialog"]')).not.toBeVisible({ timeout: 5000 });
    expect(captured?.name).toBe('Cool New App');
    expect(captured?.redirect_uris).toEqual(['https://example.com/cb', 'https://example.com/cb2']);
  });

  test('App löschen öffnet Confirm-Dialog', async ({ page }) => {
    await page.goto('/dashboard/apps');
    await page.getByRole('button', { name: /^delete$/i }).first().click();
    await expect(page.locator('[role="dialog"]')).toBeVisible();
  });

  test('client_id wird in App-Details angezeigt', async ({ page }) => {
    await page.goto('/dashboard/apps');
    await page.getByRole('button', { name: /show details/i }).first().click();
    await expect(page.getByText('cid_123')).toBeVisible();
  });

  test('client_secret wird in App-Details angezeigt (nach Show)', async ({ page }) => {
    await page.goto('/dashboard/apps');
    await page.getByRole('button', { name: /show details/i }).first().click();
    // secret is masked by default
    await expect(page.getByText('••••••••••••••••')).toBeVisible();
    // Click the Show button to reveal secret (first "Show" button is for secret toggle)
    await page.getByRole('button', { name: /^show$/i }).click();
    await expect(page.getByText('secret_abc_123')).toBeVisible();
  });

  test('Regenerate Secret Button öffnet Confirm-Dialog', async ({ page }) => {
    await page.goto('/dashboard/apps');
    await page.getByRole('button', { name: /show details/i }).first().click();
    await page.getByRole('button', { name: /regenerate/i }).click();
    await expect(page.locator('[role="dialog"]')).toBeVisible();
    await expect(page.getByRole('heading', { name: /regenerate secret/i })).toBeVisible();
  });

  test('Request Stats Widget wird in App-Details angezeigt', async ({ page }) => {
    await page.goto('/dashboard/apps');
    await page.getByRole('button', { name: /show details/i }).first().click();
    await expect(page.getByText(/statistics/i)).toBeVisible();
    // stats mock gives requests_today: 42
    await expect(page.getByText('42')).toBeVisible({ timeout: 5000 });
    await expect(page.getByText(/today/i).first()).toBeVisible();
    await expect(page.getByText(/last 7 days/i)).toBeVisible();
    await expect(page.getByText(/last 30 days/i)).toBeVisible();
  });

  test('Dashboard-Navigation zeigt Apps und Analytics Links', async ({ page }) => {
    await page.goto('/dashboard/apps');
    await expect(page.getByRole('link', { name: /^apps$/i })).toBeVisible();
    await expect(page.getByRole('link', { name: /^analytics$/i })).toBeVisible();
  });

  test('Dashboard ist ohne Auth nicht erreichbar', async ({ page }) => {
    await page.evaluate(() => {
      localStorage.removeItem('rxforge_token');
      localStorage.removeItem('rxforge_user');
    });
    await page.goto('/dashboard/apps');
    await page.waitForURL(/\/login/, { timeout: 5000 });
    await expect(page).toHaveURL(/\/login/);
  });

  test('RxForge Logo ist in der Navigation sichtbar', async ({ page }) => {
    await page.goto('/dashboard/apps');
    await expect(page.getByText('RxForge').first()).toBeVisible();
  });
});

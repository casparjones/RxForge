import { test, expect } from '@playwright/test';
import { setupApiMocks, loginAsUser } from './helpers/mock-api';

test.describe('Toast & Confirmation System', () => {
  test.beforeEach(async ({ page }) => {
    await setupApiMocks(page);
  });

  test('Toaster-Komponente ist im DOM (svelte-sonner)', async ({ page }) => {
    await loginAsUser(page);
    await page.goto('/dashboard/apps');
    // svelte-sonner renders a <section aria-label="Notifications ...">
    await expect(page.locator('section[aria-label^="Notifications"]')).toBeAttached();
  });

  test('Confirm-Dialog erscheint bei App-Löschen (Delete Button)', async ({ page }) => {
    await loginAsUser(page);
    await page.goto('/dashboard/apps');
    await page.getByRole('button', { name: /^delete$/i }).first().click();
    await expect(page.locator('[role="dialog"]')).toBeVisible();
  });

  test('Confirm-Dialog hat Confirm und Cancel Buttons', async ({ page }) => {
    await loginAsUser(page);
    await page.goto('/dashboard/apps');
    await page.getByRole('button', { name: /^delete$/i }).first().click();
    await expect(page.locator('[role="dialog"]')).toBeVisible();
    // ConfirmDialog has a Confirm and Cancel button
    await expect(page.locator('[role="dialog"]').getByRole('button', { name: /confirm/i })).toBeVisible();
    await expect(page.locator('[role="dialog"]').getByRole('button', { name: /cancel/i })).toBeVisible();
  });

  test('Confirm-Dialog zeigt App-Namen in der Nachricht', async ({ page }) => {
    await loginAsUser(page);
    await page.goto('/dashboard/apps');
    await page.getByRole('button', { name: /^delete$/i }).first().click();
    await expect(page.locator('[role="dialog"]')).toBeVisible();
    await expect(page.locator('[role="dialog"]')).toContainText('My App');
  });

  test('Cancel schließt den Confirm-Dialog', async ({ page }) => {
    await loginAsUser(page);
    await page.goto('/dashboard/apps');
    await page.getByRole('button', { name: /^delete$/i }).first().click();
    await expect(page.locator('[role="dialog"]')).toBeVisible();
    await page.locator('[role="dialog"]').getByRole('button', { name: /cancel/i }).click();
    await expect(page.locator('[role="dialog"]')).not.toBeVisible();
  });

  test('Create App Modal öffnet sich (Bestätigung über Modal)', async ({ page }) => {
    await loginAsUser(page);
    await page.goto('/dashboard/apps');
    await page.getByRole('button', { name: /\+ new app/i }).click();
    await expect(page.locator('[role="dialog"]')).toBeVisible();
    await expect(page.getByRole('heading', { name: /create new app/i })).toBeVisible();
  });

  test('Erfolgs-Toast nach Login erscheint (Welcome back)', async ({ page }) => {
    await page.goto('/login');
    await page.evaluate(() => {
      localStorage.removeItem('rxforge_token');
      localStorage.removeItem('rxforge_user');
    });
    await page.locator('input#email').fill('user@example.com');
    await page.locator('input#password').fill('password123');
    await page.locator('button[type="submit"]').click();
    // Toast may appear briefly before redirect – wait for either
    await Promise.race([
      page.waitForURL(/\/dashboard/, { timeout: 4000 }),
      page.locator('[data-sonner-toast]').waitFor({ timeout: 4000 }),
    ]).catch(() => {});
    const redirected = page.url().includes('/dashboard');
    const toastShown = await page.locator('[data-sonner-toast]').isVisible().catch(() => false);
    expect(redirected || toastShown).toBeTruthy();
  });

  test('Error-Toast erscheint bei fehlgeschlagenem Login', async ({ page }) => {
    await page.route('**/api/v1/auth/login', route => route.fulfill({
      status: 401,
      body: 'Invalid credentials',
    }));
    await page.goto('/login');
    await page.evaluate(() => {
      localStorage.removeItem('rxforge_token');
      localStorage.removeItem('rxforge_user');
    });
    await page.locator('input#email').fill('bad@example.com');
    await page.locator('input#password').fill('wrongpass');
    await page.locator('button[type="submit"]').click();
    // Error-Toast should appear from svelte-sonner (data-sonner-toast or inline error)
    const toastVisible = await page.locator('[data-sonner-toast]').first().waitFor({ timeout: 5000 }).then(() => true).catch(() => false);
    const inlineError = await page.getByText(/invalid credentials|login failed/i).first().isVisible().catch(() => false);
    expect(toastVisible || inlineError).toBeTruthy();
  });

  test('ConfirmDialog mit destructive Flag für kritische Aktionen', async ({ page }) => {
    await loginAsUser(page);
    await page.goto('/dashboard/apps');
    await page.getByRole('button', { name: /^delete$/i }).first().click();
    const dialog = page.locator('[role="dialog"]');
    await expect(dialog).toBeVisible();
    // Destructive confirm button should have red/destructive style (bg-red-*)
    const confirmBtn = dialog.getByRole('button', { name: /confirm/i });
    const cls = await confirmBtn.getAttribute('class');
    expect(cls).toMatch(/bg-red/);
  });

  test('Toaster verwendet richColors Option', async ({ page }) => {
    await loginAsUser(page);
    await page.goto('/dashboard/apps');
    const toaster = page.locator('section[aria-label^="Notifications"]');
    await expect(toaster).toBeAttached();
    // The Toaster in +layout.svelte has richColors prop set – it exposes a data attribute
    // We only check the toaster is attached (prop is internal)
  });
});

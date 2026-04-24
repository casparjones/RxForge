import { test, expect } from '@playwright/test';
import { setupApiMocks } from './helpers/mock-api';

test.describe('Login & Register', () => {
  test.beforeEach(async ({ page }) => {
    await setupApiMocks(page);
  });

  test('login-Seite hat Email und Passwort Felder', async ({ page }) => {
    await page.goto('/login');
    await expect(page.locator('input#email')).toBeVisible();
    await expect(page.locator('input#password')).toBeVisible();
    await expect(page.locator('button[type="submit"]')).toBeVisible();
  });

  test('register-Seite hat Email, Passwort und Bestätigungsfeld', async ({ page }) => {
    await page.goto('/register');
    await expect(page.locator('input#email')).toBeVisible();
    await expect(page.locator('input#password')).toBeVisible();
    await expect(page.locator('input#passwordConfirm')).toBeVisible();
    await expect(page.getByRole('button', { name: /create account/i })).toBeVisible();
  });

  test('unauthenticated redirect zu /login bei Zugriff auf /dashboard/apps', async ({ page }) => {
    await page.goto('/login');
    await page.evaluate(() => {
      localStorage.removeItem('rxforge_token');
      localStorage.removeItem('rxforge_user');
    });
    await page.goto('/dashboard/apps');
    await page.waitForURL(/\/login/, { timeout: 5000 });
    await expect(page).toHaveURL(/\/login/);
  });

  test('erfolgreicher Login redirectet zu /dashboard', async ({ page }) => {
    await page.goto('/login');
    await page.evaluate(() => {
      localStorage.removeItem('rxforge_token');
    });
    await page.locator('input#email').fill('user@example.com');
    await page.locator('input#password').fill('password123');
    await page.locator('button[type="submit"]').click();
    await page.waitForURL(/\/dashboard/, { timeout: 5000 });
    await expect(page).toHaveURL(/\/dashboard/);
  });

  test('Formularvalidierung – leere Felder verhindern Absenden', async ({ page }) => {
    await page.goto('/login');
    // Input has `required` attribute, browser HTML5 validation blocks submission
    await page.locator('button[type="submit"]').click();
    // Stay on /login
    const url = page.url();
    expect(url).toMatch(/\/login/);
    // Input validity should be false (required)
    const emailValidity = await page.locator('input#email').evaluate((el: HTMLInputElement) => el.validity.valueMissing);
    expect(emailValidity).toBe(true);
  });

  test('Formularvalidierung – ungültige Email wird abgelehnt', async ({ page }) => {
    await page.goto('/login');
    await page.locator('input#email').fill('not-an-email');
    await page.locator('input#password').fill('password123');
    await page.locator('button[type="submit"]').click();
    // The JS validator shows 'Please enter a valid email address.' (or HTML5 type=email invalidates)
    const stillOnLogin = page.url().match(/\/login/);
    expect(stillOnLogin).toBeTruthy();
  });

  test('Register-Formularvalidierung – Passwörter müssen übereinstimmen', async ({ page }) => {
    await page.goto('/register');
    await page.locator('input#email').fill('new@example.com');
    await page.locator('input#password').fill('password123');
    await page.locator('input#passwordConfirm').fill('different456');
    await page.getByRole('button', { name: /create account/i }).click();
    // Error message should appear (either inline mismatch hint or the main error banner)
    await expect(page.getByText(/passwords do not match/i).first()).toBeVisible({ timeout: 3000 });
    expect(page.url()).toMatch(/\/register/);
  });

  test('Register-Formularvalidierung – Passwort mindestens 8 Zeichen', async ({ page }) => {
    await page.goto('/register');
    await page.locator('input#email').fill('new@example.com');
    await page.locator('input#password').fill('short');
    await page.locator('input#passwordConfirm').fill('short');
    await page.getByRole('button', { name: /create account/i }).click();
    await expect(page.getByText(/at least 8 characters/i)).toBeVisible({ timeout: 3000 });
  });

  test('login-Seite hat Passkey-Button', async ({ page }) => {
    await page.goto('/login');
    await expect(page.getByRole('button', { name: /passkey/i })).toBeVisible();
  });

  test('2FA-Formular erscheint nach Login mit requires_2fa', async ({ page }) => {
    await page.route('**/api/v1/auth/login', route => route.fulfill({
      json: { requires_2fa: true, temp_token: 'temp-123', user: { id: '1', email: 'user@example.com' } }
    }));
    await page.goto('/login');
    await page.locator('input#email').fill('user@example.com');
    await page.locator('input#password').fill('password123');
    await page.locator('button[type="submit"]').click();
    await expect(page.getByText(/two-factor authentication/i)).toBeVisible();
    await expect(page.locator('input#totp')).toBeVisible();
  });

  test('2FA-Formular hat 6-stelligen Code Input mit maxlength', async ({ page }) => {
    await page.route('**/api/v1/auth/login', route => route.fulfill({
      json: { requires_2fa: true, temp_token: 'temp-123', user: { id: '1', email: 'user@example.com' } }
    }));
    await page.goto('/login');
    await page.locator('input#email').fill('user@example.com');
    await page.locator('input#password').fill('password123');
    await page.locator('button[type="submit"]').click();
    const totp = page.locator('input#totp');
    await expect(totp).toBeVisible();
    await expect(totp).toHaveAttribute('maxlength', '6');
  });

  test('register-Link auf login-Seite ist sichtbar', async ({ page }) => {
    await page.goto('/login');
    await expect(page.getByRole('link', { name: /register/i })).toBeVisible();
  });

  test('login-Link auf register-Seite ist sichtbar', async ({ page }) => {
    await page.goto('/register');
    await expect(page.getByRole('link', { name: /sign in/i })).toBeVisible();
  });

  test('Erfolgreiche Registrierung redirectet zu /dashboard', async ({ page }) => {
    await page.goto('/register');
    await page.locator('input#email').fill('new@example.com');
    await page.locator('input#password').fill('password123');
    await page.locator('input#passwordConfirm').fill('password123');
    await page.getByRole('button', { name: /create account/i }).click();
    await page.waitForURL(/\/dashboard/, { timeout: 5000 });
    await expect(page).toHaveURL(/\/dashboard/);
  });

  test('JWT wird nach erfolgreichem Login in localStorage gespeichert', async ({ page }) => {
    await page.goto('/login');
    await page.evaluate(() => localStorage.removeItem('rxforge_token'));
    await page.locator('input#email').fill('user@example.com');
    await page.locator('input#password').fill('password123');
    await page.locator('button[type="submit"]').click();
    await page.waitForURL(/\/dashboard/, { timeout: 5000 });
    const token = await page.evaluate(() => localStorage.getItem('rxforge_token'));
    expect(token).toBeTruthy();
  });
});

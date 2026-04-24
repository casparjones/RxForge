import { Page } from '@playwright/test';

export const mockUsers = [
  { id: '1', email: 'admin@example.com', role: 'admin', permissions: ['read'], locked: false },
  { id: '2', email: 'user@example.com', role: 'user', permissions: [], locked: false },
  { id: '3', email: 'super@example.com', role: 'superadmin', permissions: ['manage_users'], locked: false },
];

export const mockApps = [
  { id: 'app1', name: 'My App', client_id: 'cid_123', client_secret: 'secret_abc_123', redirect_uris: ['http://localhost:3000/callback'] },
];

export const mockGlobalAnalytics = {
  total_requests: 1234,
  active_apps: 5,
  active_users: 12,
  daily_requests: [
    { date: '2026-04-17', count: 100 },
    { date: '2026-04-18', count: 200 },
    { date: '2026-04-19', count: 150 },
  ],
  top_apps: [
    { name: 'My App', requests: 800 },
    { name: 'Other App', requests: 434 },
  ],
};

export const mockAppAnalytics = {
  requests_today: 42,
  requests_7d: 300,
  requests_30d: 1200,
  daily_requests: [
    { date: '2026-04-17', count: 30 },
    { date: '2026-04-18', count: 45 },
    { date: '2026-04-19', count: 38 },
    { date: '2026-04-20', count: 55 },
    { date: '2026-04-21', count: 42 },
    { date: '2026-04-22', count: 60 },
    { date: '2026-04-23', count: 30 },
  ],
};

export async function setupApiMocks(page: Page) {
  // Auth
  await page.route('**/api/v1/auth/login', route => route.fulfill({
    json: { token: 'mock-jwt-token', user: { id: '1', email: 'user@example.com', role: 'user' } }
  }));
  await page.route('**/api/v1/auth/register', route => route.fulfill({
    json: { token: 'mock-jwt-token', user: { id: '2', email: 'new@example.com', role: 'user' } }
  }));
  await page.route('**/api/v1/auth/logout', route => route.fulfill({ json: {} }));

  // Apps
  await page.route('**/api/v1/apps', route => {
    if (route.request().method() === 'GET') {
      route.fulfill({ json: mockApps });
    } else {
      route.fulfill({ json: { id: 'new-app', name: 'New App', client_id: 'cid_new', client_secret: 'secret_new', redirect_uris: [] } });
    }
  });
  await page.route('**/api/v1/apps/*', route => {
    const method = route.request().method();
    if (method === 'DELETE') {
      route.fulfill({ status: 204, body: '' });
    } else {
      route.fulfill({ json: mockApps[0] });
    }
  });
  await page.route('**/api/v1/apps/*/regenerate-secret', route => route.fulfill({
    json: { client_secret: 'secret_regenerated_xyz' }
  }));

  // Admin
  await page.route('**/api/v1/admin/users', route => route.fulfill({ json: mockUsers }));
  await page.route('**/api/v1/admin/users/*/role', route => route.fulfill({ json: { ok: true } }));
  await page.route('**/api/v1/admin/users/*/permissions', route => route.fulfill({ json: { ok: true } }));
  await page.route('**/api/v1/admin/users/*/lock', route => route.fulfill({ json: { ok: true } }));

  // Analytics
  await page.route('**/api/v1/analytics/global', route => route.fulfill({ json: mockGlobalAnalytics }));
  await page.route('**/api/v1/analytics/apps/**', route => route.fulfill({ json: mockAppAnalytics }));
}

/**
 * Log in through the actual login form so auth store gets hydrated.
 * Required because the store does not load `user` from localStorage on startup —
 * admin layout relies on $auth.user.role, which is only set via auth.login().
 */
export async function loginAsUser(page: Page, opts: { email?: string; role?: string; id?: string } = {}) {
  const email = opts.email ?? 'user@example.com';
  const role = opts.role ?? 'user';
  const id = opts.id ?? (role === 'admin' ? '1' : role === 'superadmin' ? '3' : '2');
  // Override login route for this session
  await page.route('**/api/v1/auth/login', route => route.fulfill({
    json: { token: 'mock-jwt-token-' + role, user: { id, email, role } }
  }));
  await page.goto('/login');
  await page.locator('input#email').fill(email);
  await page.locator('input#password').fill('password123');
  await page.locator('button[type="submit"]').click();
  await page.waitForURL(/\/dashboard/, { timeout: 5000 });
}

export async function loginAsAdmin(page: Page) {
  await loginAsUser(page, { email: 'admin@example.com', role: 'admin', id: '1' });
}

export async function loginAsSuperadmin(page: Page) {
  await loginAsUser(page, { email: 'super@example.com', role: 'superadmin', id: '3' });
}

/**
 * Client-side navigation from an already-loaded SvelteKit page. Uses an
 * injected anchor with data-sveltekit-reload=false so SvelteKit intercepts
 * the navigation via its client router, preserving the in-memory auth store
 * (which otherwise resets on full reload since the store does not rehydrate
 * `user` from localStorage).
 */
export async function gotoSPA(page: Page, path: string) {
  await page.evaluate(async (p) => {
    const a = document.createElement('a');
    a.href = p;
    a.textContent = 'spa-nav';
    a.setAttribute('data-test-spa-nav', '1');
    document.body.appendChild(a);
    a.click();
    a.remove();
  }, path);
  const re = new RegExp(path.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'));
  await page.waitForURL(re, { timeout: 5000 });
}

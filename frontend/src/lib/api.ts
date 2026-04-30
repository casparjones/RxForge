const BASE = '/api/v1';

async function fetchApi<T>(path: string, options?: RequestInit): Promise<T> {
  const token = localStorage.getItem('rxforge_token');
  const res = await fetch(BASE + path, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...options?.headers,
    },
  });

  if (res.status === 401) {
    localStorage.removeItem('rxforge_token');
    localStorage.removeItem('rxforge_user');
    window.location.href = '/login';
    throw new Error('Unauthorized');
  }

  if (!res.ok) {
    const text = await res.text();
    let message = text;
    try { const json = JSON.parse(text); if (json.error) message = json.error; } catch {}
    throw new Error(message);
  }
  return res.json();
}

export const api = {
  auth: {
    login: (email: string, password: string) =>
      fetchApi<{ token: string; user: any }>('/auth/login', { method: 'POST', body: JSON.stringify({ email, password }) }),
    register: (email: string, password: string, invite_code?: string) =>
      fetchApi<{ token: string; user: any }>('/auth/register', { method: 'POST', body: JSON.stringify({ email, password, invite_code }) }),
    logout: () => fetchApi('/auth/logout', { method: 'POST' }),
    info: () => fetch('/api/v1/auth/info').then(r => r.json() as Promise<{ invite_required: boolean }>),
  },
  apps: {
    list: () => fetchApi<any[]>('/apps'),
    get: (id: string) => fetchApi<any>(`/apps/${id}`),
    create: (data: { name: string; redirect_uris: string[]; auth_type?: string; db_scope?: string }) =>
      fetchApi<any>('/apps', { method: 'POST', body: JSON.stringify(data) }),
    update: (id: string, data: { name?: string; redirect_uris?: string[]; auth_type?: string; db_scope?: string }) =>
      fetchApi<any>(`/apps/${id}`, { method: 'PATCH', body: JSON.stringify(data) }),
    delete: (id: string) => fetchApi(`/apps/${id}`, { method: 'DELETE' }),
    regenerateSecret: (id: string) => fetchApi(`/apps/${id}/regenerate-secret`, { method: 'POST' }),
    getStats: (id: string) => fetchApi<any>(`/analytics/apps/${id}`),
    tokens: {
      list: (appId: string) => fetchApi<any[]>(`/apps/${appId}/tokens`),
      create: (appId: string, data: { name?: string; allowed_origins?: string[] }) =>
        fetchApi<any>(`/apps/${appId}/tokens`, { method: 'POST', body: JSON.stringify(data) }),
      revoke: (appId: string, tokenId: string) =>
        fetchApi(`/apps/${appId}/tokens/${tokenId}`, { method: 'DELETE' }),
      update: (appId: string, tokenId: string, data: { name?: string; allowed_origins?: string[] }) =>
        fetchApi(`/apps/${appId}/tokens/${tokenId}`, { method: 'PATCH', body: JSON.stringify(data) }),
      purge: (appId: string, tokenId: string) =>
        fetchApi(`/apps/${appId}/tokens/${tokenId}/purge`, { method: 'DELETE' }),
    },
    db: {
      list: (appId: string, page = 1, perPage = 20) =>
        fetchApi<{ docs: any[]; total: number; page: number; per_page: number; pages: number }>(
          `/apps/${appId}/db/docs?page=${page}&per_page=${perPage}`
        ),
      getDoc: (appId: string, docId: string) =>
        fetchApi<any>(`/apps/${appId}/db/docs/${encodeURIComponent(docId)}`),
      updateDoc: (appId: string, docId: string, body: any) =>
        fetchApi<any>(`/apps/${appId}/db/docs/${encodeURIComponent(docId)}`, {
          method: 'PUT', body: JSON.stringify(body),
        }),
      deleteDoc: (appId: string, docId: string, rev: string) =>
        fetchApi<any>(`/apps/${appId}/db/docs/${encodeURIComponent(docId)}?rev=${encodeURIComponent(rev)}`, {
          method: 'DELETE',
        }),
      deleteAll: (appId: string) =>
        fetchApi<{ deleted: number }>(`/apps/${appId}/db/docs`, { method: 'DELETE' }),
    },
  },
  admin: {
    users: {
      list: () => fetchApi<any[]>('/admin/users'),
      apps: (id: string) => fetchApi<any[]>(`/admin/users/${id}/apps`),
      updateRole: (id: string, role: string) =>
        fetchApi(`/admin/users/${id}/role`, { method: 'PATCH', body: JSON.stringify({ role }) }),
      updatePermissions: (id: string, permissions: string[]) =>
        fetchApi(`/admin/users/${id}/permissions`, { method: 'PUT', body: JSON.stringify({ permissions }) }),
      setLocked: (id: string, locked: boolean) =>
        fetchApi(`/admin/users/${id}/lock`, { method: 'PUT', body: JSON.stringify({ locked }) }),
    },
    analytics: { global: () => fetchApi<any>('/analytics/global') },
  },
  me: {
    stats: () => fetchApi<{ last_login_at: string | null; app_count: number; granted_rights_count: number }>('/auth/me/stats'),
  },
  oauth: {
    clientInfo: (client_id: string) =>
      fetch(`/oauth/client-info?client_id=${encodeURIComponent(client_id)}`).then(r => r.ok ? r.json() : Promise.reject(new Error('Client not found'))),
    consentCheck: (client_id: string) =>
      fetchApi<{ consented: boolean }>(`/oauth/consent/check?client_id=${encodeURIComponent(client_id)}`),
    consentGrant: (data: { client_id: string; redirect_uri: string; scope?: string; state?: string }) =>
      fetchApi<{ redirect_url: string }>('/oauth/consent', { method: 'POST', body: JSON.stringify(data) }),
  },
  rights: {
    list: () => fetchApi<{ client_id: string; app_name: string; granted_at: string }[]>('/oauth/rights'),
    revoke: (client_id: string) => fetchApi(`/oauth/rights/${encodeURIComponent(client_id)}`, { method: 'DELETE' }),
  },
};

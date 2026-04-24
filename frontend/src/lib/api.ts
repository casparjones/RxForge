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
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export const api = {
  auth: {
    login: (email: string, password: string) =>
      fetchApi<{ token: string; user: any }>('/auth/login', { method: 'POST', body: JSON.stringify({ email, password }) }),
    register: (email: string, password: string) =>
      fetchApi<{ token: string; user: any }>('/auth/register', { method: 'POST', body: JSON.stringify({ email, password }) }),
    logout: () => fetchApi('/auth/logout', { method: 'POST' }),
  },
  apps: {
    list: () => fetchApi<any[]>('/apps'),
    create: (data: { name: string; redirect_uris: string[] }) =>
      fetchApi<any>('/apps', { method: 'POST', body: JSON.stringify(data) }),
    delete: (id: string) => fetchApi(`/apps/${id}`, { method: 'DELETE' }),
    regenerateSecret: (id: string) => fetchApi(`/apps/${id}/regenerate-secret`, { method: 'POST' }),
    getStats: (id: string) => fetchApi<any>(`/analytics/apps/${id}`),
  },
  admin: {
    users: {
      list: () => fetchApi<any[]>('/admin/users'),
      updateRole: (id: string, role: string) =>
        fetchApi(`/admin/users/${id}/role`, { method: 'PUT', body: JSON.stringify({ role }) }),
      updatePermissions: (id: string, permissions: string[]) =>
        fetchApi(`/admin/users/${id}/permissions`, { method: 'PUT', body: JSON.stringify({ permissions }) }),
      setLocked: (id: string, locked: boolean) =>
        fetchApi(`/admin/users/${id}/lock`, { method: 'PUT', body: JSON.stringify({ locked }) }),
    },
    analytics: { global: () => fetchApi<any>('/analytics/global') },
  },
};

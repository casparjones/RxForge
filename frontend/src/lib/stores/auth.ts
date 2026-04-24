import { writable, derived } from 'svelte/store';

interface AuthState {
  token: string | null;
  user: { id: string; email: string; role: string } | null;
}

function readStoredUser(): AuthState['user'] {
  if (typeof localStorage === 'undefined') return null;
  const raw = localStorage.getItem('rxforge_user');
  if (!raw) return null;
  try {
    return JSON.parse(raw) as AuthState['user'];
  } catch {
    return null;
  }
}

function createAuthStore() {
  const { subscribe, set } = writable<AuthState>({
    token: typeof localStorage !== 'undefined' ? localStorage.getItem('rxforge_token') : null,
    user: readStoredUser(),
  });
  return {
    subscribe,
    login: (token: string, user: AuthState['user']) => {
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem('rxforge_token', token);
        if (user) localStorage.setItem('rxforge_user', JSON.stringify(user));
        else localStorage.removeItem('rxforge_user');
      }
      set({ token, user });
    },
    logout: () => {
      if (typeof localStorage !== 'undefined') {
        localStorage.removeItem('rxforge_token');
        localStorage.removeItem('rxforge_user');
      }
      set({ token: null, user: null });
    },
  };
}

export const auth = createAuthStore();
export const isAuthenticated = derived(auth, $auth => !!$auth.token);
export const isAdmin = derived(auth, $auth => $auth.user?.role === 'admin' || $auth.user?.role === 'superadmin');

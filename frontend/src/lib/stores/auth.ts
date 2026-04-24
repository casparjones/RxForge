import { writable, derived } from 'svelte/store';

interface AuthState {
  token: string | null;
  user: { id: string; email: string; role: string } | null;
}

function createAuthStore() {
  const { subscribe, set, update } = writable<AuthState>({
    token: typeof localStorage !== 'undefined' ? localStorage.getItem('rxforge_token') : null,
    user: null,
  });
  return {
    subscribe,
    login: (token: string, user: AuthState['user']) => {
      localStorage.setItem('rxforge_token', token);
      set({ token, user });
    },
    logout: () => {
      localStorage.removeItem('rxforge_token');
      set({ token: null, user: null });
    },
  };
}

export const auth = createAuthStore();
export const isAuthenticated = derived(auth, $auth => !!$auth.token);
export const isAdmin = derived(auth, $auth => $auth.user?.role === 'admin' || $auth.user?.role === 'superadmin');

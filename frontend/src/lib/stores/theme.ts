import { writable } from 'svelte/store';

type Theme = 'light' | 'dark';

function getInitialTheme(): Theme {
  if (typeof localStorage === 'undefined') return 'light';
  const stored = localStorage.getItem('rxforge_theme') as Theme | null;
  if (stored === 'dark' || stored === 'light') return stored;
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

function applyTheme(t: Theme) {
  if (typeof document !== 'undefined') {
    document.documentElement.setAttribute('data-theme', t === 'dark' ? 'dim' : 'light');
  }
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem('rxforge_theme', t);
  }
}

function createThemeStore() {
  const { subscribe, set, update } = writable<Theme>(getInitialTheme());

  return {
    subscribe,
    init: () => {
      const t = getInitialTheme();
      applyTheme(t);
      set(t);
    },
    toggle: () => update(t => {
      const next: Theme = t === 'dark' ? 'light' : 'dark';
      applyTheme(next);
      return next;
    }),
    set: (t: Theme) => {
      applyTheme(t);
      set(t);
    },
  };
}

export const theme = createThemeStore();

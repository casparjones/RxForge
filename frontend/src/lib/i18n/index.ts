import { writable, derived } from 'svelte/store';
import { en } from './en';
import { de } from './de';

export type Lang = 'en' | 'de';

const dicts: Record<Lang, Record<string, any>> = { en, de };

function getInitialLang(): Lang {
  if (typeof localStorage === 'undefined') return 'en';
  const stored = localStorage.getItem('rxforge_lang');
  if (stored === 'en' || stored === 'de') return stored;
  const browser = typeof navigator !== 'undefined' ? navigator.language.slice(0, 2) : 'en';
  return browser === 'de' ? 'de' : 'en';
}

export const lang = writable<Lang>(getInitialLang());

lang.subscribe(l => {
  if (typeof localStorage !== 'undefined') localStorage.setItem('rxforge_lang', l);
});

function deepGet(obj: Record<string, any>, path: string): string | undefined {
  const parts = path.split('.');
  let cur: any = obj;
  for (const p of parts) {
    if (cur == null || typeof cur !== 'object') return undefined;
    cur = cur[p];
  }
  return typeof cur === 'string' ? cur : undefined;
}

export const t = derived(lang, ($l) => (key: string, params?: Record<string, string | number>): string => {
  let str = deepGet(dicts[$l], key) ?? deepGet(dicts.en, key) ?? key;
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      str = str.replace(`{${k}}`, String(v));
    }
  }
  return str;
});

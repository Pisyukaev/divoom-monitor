import { invoke } from '@tauri-apps/api/core';

/**
 * Per-device LocalToken (Divoom app → Wi-Fi devices → device settings →
 * Local token). Newer Times Gate firmware needs it to return settings.
 * Persisted here; the Rust side keeps an in-memory copy that it adds to every
 * request for that IP, so it must be pushed again on each app start.
 */
const KEY_PREFIX = 'local_token_';
const MAX_TOKEN = 0xffffffff;

/** Prefix of the error `get_device_info` returns when a token is needed. */
export const TOKEN_REQUIRED_ERROR = 'LOCAL_TOKEN_REQUIRED';

export function getLocalToken(ip: string): string {
  return localStorage.getItem(KEY_PREFIX + ip) ?? '';
}

/** Digits only and fits in u32. */
export function isValidLocalToken(token: string): boolean {
  return /^\d{1,10}$/.test(token) && Number(token) <= MAX_TOKEN;
}

/** Saves (empty string clears) and sends it to the backend. */
export async function saveLocalToken(ip: string, token: string): Promise<void> {
  const trimmed = token.trim();
  if (trimmed) {
    if (!isValidLocalToken(trimmed)) throw new Error('Invalid LocalToken');
    localStorage.setItem(KEY_PREFIX + ip, trimmed);
  } else {
    localStorage.removeItem(KEY_PREFIX + ip);
  }
  await invoke('set_local_token', {
    ipAddress: ip,
    token: trimmed ? Number(trimmed) : null,
  });
}

let syncPromise: Promise<void> | null = null;

/** Pushes all saved tokens to the backend once per app run. */
export function syncLocalTokens(): Promise<void> {
  syncPromise ??= (async () => {
    const keys = Object.keys(localStorage).filter((k) => k.startsWith(KEY_PREFIX));
    for (const key of keys) {
      const token = localStorage.getItem(key) ?? '';
      if (!isValidLocalToken(token)) continue;
      try {
        await invoke('set_local_token', {
          ipAddress: key.slice(KEY_PREFIX.length),
          token: Number(token),
        });
      } catch (e) {
        console.error('Failed to restore LocalToken:', e);
      }
    }
  })();
  return syncPromise;
}

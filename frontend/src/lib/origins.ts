/** Normalize a user-entered string to a bare origin (scheme + host + optional port).
 *  "https://example.com/foo/bar" → "https://example.com"
 *  "http://localhost:3000/path"  → "http://localhost:3000"
 *  Already-bare origins are returned unchanged.
 *  Unparseable strings are returned as-is so the server can reject them explicitly.
 */
export function normalizeOrigin(value: string): string {
	const s = value.trim();
	try {
		const url = new URL(s.includes('://') ? s : `https://${s}`);
		return `${url.protocol}//${url.host}`;
	} catch {
		return s;
	}
}

export function parseOrigins(raw: string): string[] {
	return raw.split('\n').map(normalizeOrigin).filter(Boolean);
}

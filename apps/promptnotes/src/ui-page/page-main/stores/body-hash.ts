/**
 * Frontend body-hash helper for external-change conflict detection.
 *
 * Prefers Web Crypto SHA-256 (matches the Rust `BodyHash::from_body`), falling back to a
 * deterministic plain token when `crypto.subtle` is unavailable (non-secure context). Both the
 * local and disk body go through the same function, so equality comparison stays correct either way.
 */
export async function hashBody(body: string): Promise<string> {
	const subtle = globalThis.crypto?.subtle;
	if (!subtle) return `plain:${body}`;
	const digest = await subtle.digest('SHA-256', new TextEncoder().encode(body));
	return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, '0')).join('');
}

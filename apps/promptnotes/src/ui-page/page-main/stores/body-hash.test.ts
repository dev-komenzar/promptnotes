import { describe, expect, it } from 'vitest';
import { hashBody } from './body-hash';

describe('page-main:body-hash', () => {
	it('matches the known SHA-256 vectors used by the Rust BodyHash', async () => {
		await expect(hashBody('')).resolves.toBe(
			'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'
		);
		await expect(hashBody('abc')).resolves.toBe(
			'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad'
		);
	});

	it('is deterministic and distinguishes different bodies', async () => {
		const a = await hashBody('hello local');
		const b = await hashBody('hello local');
		const c = await hashBody('hello world');
		expect(a).toBe(b);
		expect(a).not.toBe(c);
	});
});

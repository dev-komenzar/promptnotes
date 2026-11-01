import { describe, expect, it, vi } from 'vitest';
import {
	createUpdateToastStore,
	type NewVersionDetectedPayload,
	type UpdateEventListener
} from '../store.svelte';

function makePayload(
	overrides: Partial<NewVersionDetectedPayload> = {}
): NewVersionDetectedPayload {
	return {
		current_version: '1.3.2',
		latest_version: '1.4.0',
		release_url: 'https://github.com/dev-komenzar/promptnotes/releases/tag/v1.4.0',
		release_notes: '- bug fix: 起動時 crash\n- feature: 検索の正規化',
		...overrides
	};
}

type Capture = {
	emit: (payload: NewVersionDetectedPayload) => void;
	unsubscribe: ReturnType<typeof vi.fn>;
	listenFn: UpdateEventListener;
};

function makeListenCapture(): Capture {
	let stored: ((payload: NewVersionDetectedPayload) => void) | null = null;
	const unsubscribe = vi.fn();
	const listenFn: UpdateEventListener = async (handler) => {
		stored = handler;
		return unsubscribe;
	};
	return {
		emit: (payload) => {
			if (!stored) throw new Error('listenFn was not awaited before emit');
			stored(payload);
		},
		unsubscribe,
		listenFn
	};
}

describe('widget:widget-update-toast store', () => {
	it('spec#tp-ut-no-mount-without-event — event 未受信時は payload=null', async () => {
		const cap = makeListenCapture();
		const openUrlFn = vi.fn();
		const store = createUpdateToastStore({ listenFn: cap.listenFn, openUrlFn });

		await store.start();

		expect(store.payload).toBeNull();
		expect(openUrlFn).not.toHaveBeenCalled();
	});

	it('spec#tp-ut-event-mounts-toast — NewVersionDetected で payload が set される', async () => {
		const cap = makeListenCapture();
		const store = createUpdateToastStore({ listenFn: cap.listenFn, openUrlFn: vi.fn() });

		await store.start();
		const payload = makePayload();
		cap.emit(payload);

		expect(store.payload).toStrictEqual(payload);
	});

	it('spec#tp-ut-latest-overrides (I-UT3) — 再 emit は最新で上書き', async () => {
		const cap = makeListenCapture();
		const store = createUpdateToastStore({ listenFn: cap.listenFn, openUrlFn: vi.fn() });

		await store.start();
		cap.emit(makePayload({ latest_version: '1.4.0' }));
		cap.emit(makePayload({ latest_version: '1.4.1', release_url: 'https://example.com/v1.4.1' }));

		expect(store.payload?.latest_version).toBe('1.4.1');
		expect(store.payload?.release_url).toBe('https://example.com/v1.4.1');
	});

	it('spec#tp-ut-dismiss-clears — dismiss で payload=null', async () => {
		const cap = makeListenCapture();
		const openUrlFn = vi.fn();
		const store = createUpdateToastStore({ listenFn: cap.listenFn, openUrlFn });

		await store.start();
		cap.emit(makePayload());
		expect(store.payload).not.toBeNull();

		store.dismiss();

		expect(store.payload).toBeNull();
		expect(openUrlFn).not.toHaveBeenCalled();
	});

	it('spec#tp-ut-redisplay-on-newer (I-UT4 / I-UT3) — dismiss 後に定期確認で新しいバージョンが届くと再表示', async () => {
		const cap = makeListenCapture();
		const store = createUpdateToastStore({ listenFn: cap.listenFn, openUrlFn: vi.fn() });

		await store.start();
		// 起動時の確認で 1.4.0 を受信 → dismiss
		cap.emit(makePayload({ latest_version: '1.4.0' }));
		store.dismiss();
		expect(store.payload).toBeNull();

		// 24 時間後の定期確認で未通知の 1.4.1 が届く (同一バージョンは発行側が抑止する、I-U4)
		cap.emit(makePayload({ latest_version: '1.4.1', release_url: 'https://example.com/v1.4.1' }));

		expect(store.payload?.latest_version).toBe('1.4.1');
		expect(store.payload?.release_url).toBe('https://example.com/v1.4.1');
	});

	it('spec#tp-ut-view-release-opens — view-release で openUrlFn を 1 回呼び payload は維持', async () => {
		const cap = makeListenCapture();
		const openUrlFn = vi.fn();
		const store = createUpdateToastStore({ listenFn: cap.listenFn, openUrlFn });

		await store.start();
		const payload = makePayload();
		cap.emit(payload);

		await store.viewRelease();

		expect(openUrlFn).toHaveBeenCalledExactlyOnceWith(payload.release_url);
		expect(store.payload).toStrictEqual(payload);
	});

	it('spec#tp-ut-view-release-opens (payload null guard) — payload 未 set 時は何もしない', async () => {
		const cap = makeListenCapture();
		const openUrlFn = vi.fn();
		const store = createUpdateToastStore({ listenFn: cap.listenFn, openUrlFn });

		await store.start();
		await store.viewRelease();

		expect(openUrlFn).not.toHaveBeenCalled();
	});

	it('spec#tp-ut-stop-unsubscribes — stop で listenFn が返した unsubscribe が 1 回呼ばれる', async () => {
		const cap = makeListenCapture();
		const store = createUpdateToastStore({ listenFn: cap.listenFn, openUrlFn: vi.fn() });

		await store.start();
		store.stop();

		expect(cap.unsubscribe).toHaveBeenCalledTimes(1);
	});

	it('spec#tp-ut-listen-failure-silent — listenFn が reject しても throw せず payload=null', async () => {
		const failingListen: UpdateEventListener = async () => {
			throw new Error('boom');
		};
		const openUrlFn = vi.fn();
		const store = createUpdateToastStore({ listenFn: failingListen, openUrlFn });

		await expect(store.start()).resolves.toBeUndefined();
		expect(store.payload).toBeNull();
	});

	it('spec#invariants-lifecycle (I-UT1) — start を 2 回呼んでも listenFn は 1 回のみ', async () => {
		const listenFn = vi.fn<UpdateEventListener>(async () => () => {});
		const store = createUpdateToastStore({ listenFn, openUrlFn: vi.fn() });

		await store.start();
		await store.start();

		expect(listenFn).toHaveBeenCalledTimes(1);
	});
});

describe('widget:widget-update-toast startup check hook (ori-m2ra.1)', () => {
	it('spec#tp-ut-subscribed-hook (I-UT8) — onSubscribed は listen 確立後に 1 回だけ呼ばれ、その中の emit を取りこぼさない', async () => {
		const cap = makeListenCapture();
		const payload = makePayload();
		// onSubscribed の中で即 emit する = 起動時 check が event を emit するケース。
		// listen 確立前に呼ばれていれば cap.emit が throw し、payload も set されない。
		const onSubscribed = vi.fn(() => cap.emit(payload));
		const store = createUpdateToastStore({
			listenFn: cap.listenFn,
			openUrlFn: vi.fn(),
			onSubscribed
		});

		await store.start();
		await store.start();

		expect(onSubscribed).toHaveBeenCalledTimes(1);
		expect(store.payload).toStrictEqual(payload);
	});

	it('spec#tp-ut-listen-failure-silent — listen 失敗時は onSubscribed を呼ばない', async () => {
		const failingListen: UpdateEventListener = async () => {
			throw new Error('boom');
		};
		const onSubscribed = vi.fn();
		const store = createUpdateToastStore({
			listenFn: failingListen,
			openUrlFn: vi.fn(),
			onSubscribed
		});

		await store.start();

		expect(onSubscribed).not.toHaveBeenCalled();
	});

	it('spec#tp-ut-subscribed-hook (I-UT8 / S14) — onSubscribed が reject しても start は throw しない', async () => {
		const cap = makeListenCapture();
		const store = createUpdateToastStore({
			listenFn: cap.listenFn,
			openUrlFn: vi.fn(),
			onSubscribed: async () => {
				throw new Error('network down');
			}
		});

		await expect(store.start()).resolves.toBeUndefined();
		expect(store.payload).toBeNull();
	});
});

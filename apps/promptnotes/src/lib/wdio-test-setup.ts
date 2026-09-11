import { browser } from '$app/environment';

if (browser && import.meta.env.VITE_WDIO_TEST) {
	void import('@wdio/tauri-plugin');
}

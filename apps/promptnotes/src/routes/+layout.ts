// Tauri shell hosts a static SPA; SSR is not available and we want
// every route prerendered into the build/ directory.
export const prerender = true;
export const ssr = false;

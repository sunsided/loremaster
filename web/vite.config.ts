import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

// The build environment, without the Node type definitions.
const env = (globalThis as { process?: { env: Record<string, string | undefined> } }).process?.env ?? {};
// A static host can serve the app below a path, for example `/lantern` on GitHub Pages.
const base = (env.BASE_PATH ?? '') as '' | `/${string}`;
// GitHub Pages answers unknown paths with 404.html; other hosts use 200.html.
const fallback = env.FALLBACK ?? '200.html';

export default defineConfig({
	plugins: [
		sveltekit({
			adapter: adapter({ fallback }),
			paths: { base },
			// Prerendered pages get the policy as a meta tag. The app talks only to its own origin, to
			// `lantern serve` on this computer and to the Anthropic API. WASM needs 'wasm-unsafe-eval'.
			csp: {
				mode: 'hash',
				directives: {
					'default-src': ['self'],
					'script-src': ['self', 'wasm-unsafe-eval'],
					'style-src': ['self', 'unsafe-inline', 'https://fonts.googleapis.com'],
					'font-src': ['self', 'https://fonts.gstatic.com'],
					'img-src': ['self', 'data:', 'blob:'],
					'connect-src': ['self', 'http://127.0.0.1:4710', 'https://api.anthropic.com'],
					'worker-src': ['self', 'blob:'],
					'object-src': ['none'],
					'base-uri': ['self']
				}
			}
		})
	],
	server: {
		// The example story lives outside the web app, in ../examples.
		fs: { allow: ['..'] },
		// Trunk stages each build in pkg/.stage before swapping it in; only the
		// final files should trigger a reload.
		watch: { ignored: ['**/engine/pkg/.stage/**'] }
	}
});

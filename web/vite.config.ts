import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [
		sveltekit({
			adapter: adapter({ fallback: '200.html' })
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

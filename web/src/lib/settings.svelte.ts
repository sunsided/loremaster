// Per-viewer display settings, remembered in localStorage when it is available.

import type { EffectsLevel as Effects, MotionPreference as Motion, Theme } from '#lib/design/types.ts';

const KEY = 'lantern.settings';

interface Stored {
	theme: Theme;
	effects: Effects;
	motion: Motion;
	sources: boolean;
}

const defaults: Stored = { theme: 'phosphor', effects: 'full', motion: 'system', sources: true };

function read(): Stored {
	try {
		const raw = localStorage.getItem(KEY);
		return raw ? { ...defaults, ...JSON.parse(raw) } : defaults;
	} catch {
		return defaults;
	}
}

class Settings {
	theme = $state<Theme>(defaults.theme);
	effects = $state<Effects>(defaults.effects);
	motion = $state<Motion>(defaults.motion);
	/** Show fragment numbers in the transcript. */
	sources = $state(defaults.sources);
	/** The operating system asks for reduced motion. */
	systemReduce = $state(false);

	reduceMotion = $derived(this.motion === 'reduce' || this.systemReduce);

	constructor() {
		const s = read();
		this.theme = s.theme;
		this.effects = s.effects;
		this.motion = s.motion;
		this.sources = s.sources;
		if (typeof window !== 'undefined' && window.matchMedia) {
			const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
			this.systemReduce = mq.matches;
			mq.addEventListener('change', () => (this.systemReduce = mq.matches));
		}
		$effect.root(() => {
			$effect(() => {
				const value: Stored = {
					theme: this.theme,
					effects: this.effects,
					motion: this.motion,
					sources: this.sources
				};
				try {
					localStorage.setItem(KEY, JSON.stringify(value));
				} catch {
					// Private mode or blocked storage: settings last for this visit only.
				}
			});
		});
	}
}

export const settings = new Settings();

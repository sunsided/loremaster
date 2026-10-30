// Per-viewer display settings, remembered in localStorage when it is available.

import '#lib/migrate.ts';
import type { EffectsLevel as Effects, MotionPreference as Motion, RevealMode, Theme } from '#lib/design/types.ts';

const KEY = 'loremaster.settings';

interface Stored {
	theme: Theme;
	effects: Effects;
	motion: Motion;
	tooltips: boolean;
	reveal: RevealMode;
	serveLink: boolean;
}

const defaults: Stored = { theme: 'phosphor', effects: 'full', motion: 'system', tooltips: true, reveal: 'lines', serveLink: false };

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
	/** Show author tooltips on things and exits. */
	tooltips = $state(defaults.tooltips);
	/** How new game text appears: line by line, or a paragraph at once. */
	reveal = $state<RevealMode>(defaults.reveal);
	/** On a hosted page: look for `loremaster serve` on this computer. Off by default, because the browser asks for permission. */
	serveLink = $state(defaults.serveLink);
	/** The operating system asks for reduced motion. */
	systemReduce = $state(false);

	reduceMotion = $derived(this.motion === 'reduce' || this.systemReduce);

	constructor() {
		const s = read();
		this.theme = s.theme;
		this.effects = s.effects;
		this.motion = s.motion;
		this.tooltips = s.tooltips;
		this.reveal = s.reveal === 'blocks' ? 'blocks' : 'lines';
		this.serveLink = s.serveLink === true;
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
					tooltips: this.tooltips,
					reveal: this.reveal,
					serveLink: this.serveLink
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

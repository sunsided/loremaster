// Plays music, ambience and sound effects with the Web Audio API. The engine has no audio code: it says what is active.

export type Channel = 'music' | 'ambience' | 'effects';
type Loop = Exclude<Channel, 'effects'>;

const FADE_SECONDS = 0.8;

interface Playing {
	url: string;
	source: AudioBufferSourceNode;
	fade: GainNode;
}

export class AudioPlayer {
	#context: AudioContext | null = null;
	#gains = {} as Record<Channel, GainNode>;
	#volumes: Record<Channel, number> = { music: 1, ambience: 1, effects: 1 };
	#buffers = new Map<string, Promise<AudioBuffer>>();
	#wanted: Record<Loop, string | null> = { music: null, ambience: null };
	#playing: Record<Loop, Playing | null> = { music: null, ambience: null };

	/** Call it on the first input of the player. Before it, nothing plays. */
	unlock() {
		if (this.#context) return;
		const context = new AudioContext();
		this.#context = context;
		for (const channel of ['music', 'ambience', 'effects'] as const) {
			const gain = context.createGain();
			gain.gain.value = this.#volumes[channel];
			gain.connect(context.destination);
			this.#gains[channel] = gain;
		}
		void context.resume();
		this.#sync('music');
		this.#sync('ambience');
	}

	/** Set a volume, from 0 to 1. */
	volume(channel: Channel, value: number) {
		this.#volumes[channel] = value;
		if (this.#context) this.#gains[channel].gain.value = value;
	}

	/** The loop that should run on a channel now, or null for silence. A change crossfades. */
	loop(channel: Loop, url: string | null) {
		if (this.#wanted[channel] === url) return;
		this.#wanted[channel] = url;
		this.#sync(channel);
	}

	/** Play a sound effect once. */
	once(url: string) {
		const context = this.#context;
		if (!context) return;
		void this.#buffer(context, url)
			.then((buffer) => {
				const source = context.createBufferSource();
				source.buffer = buffer;
				source.connect(this.#gains.effects);
				source.start();
			})
			.catch((e) => console.warn(`cannot play ${url}`, e));
	}

	#buffer(context: AudioContext, url: string): Promise<AudioBuffer> {
		let buffer = this.#buffers.get(url);
		if (!buffer) {
			buffer = fetch(url)
				.then((r) => r.arrayBuffer())
				.then((data) => context.decodeAudioData(data));
			this.#buffers.set(url, buffer);
		}
		return buffer;
	}

	#sync(channel: Loop) {
		const context = this.#context;
		if (!context) return;
		const url = this.#wanted[channel];
		const old = this.#playing[channel];
		if (old?.url === url) return;
		if (old) {
			const end = context.currentTime + FADE_SECONDS;
			old.fade.gain.setValueAtTime(old.fade.gain.value, context.currentTime);
			old.fade.gain.linearRampToValueAtTime(0, end);
			old.source.stop(end);
			this.#playing[channel] = null;
		}
		if (!url) return;
		void this.#buffer(context, url)
			.then((buffer) => {
				// The wish changed while the file loaded.
				if (this.#wanted[channel] !== url || this.#playing[channel]) return;
				const source = context.createBufferSource();
				source.buffer = buffer;
				source.loop = true;
				const fade = context.createGain();
				fade.gain.setValueAtTime(0, context.currentTime);
				fade.gain.linearRampToValueAtTime(1, context.currentTime + FADE_SECONDS);
				source.connect(fade).connect(this.#gains[channel]);
				source.start();
				this.#playing[channel] = { url, source, fade };
			})
			.catch((e) => console.warn(`cannot play ${url}`, e));
	}
}

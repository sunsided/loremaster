<script lang="ts">
	import TextInput from '../atoms/TextInput.svelte';
	import { complete } from '#lib/player/complete.ts';

	interface Props {
		/** Called with the typed command when the player pushes Enter. */
		oncommand: (text: string) => void;
		placeholder?: string;
		/** The text in the prompt. Bind it to keep the prompt and other controls in sync. */
		value?: string;
		/** The input element, for focus from outside (for example a skip link). */
		input?: HTMLInputElement;
		/** The verbs that Tab completes at the start of a command, for example `take` and `pick up`. */
		verbs?: string[];
		/** The words that Tab completes after the verb, and also at the start: things in reach, exits. */
		nouns?: string[];
	}

	let {
		oncommand,
		placeholder = 'type a command, or use the verbs below',
		value = $bindable(''),
		input = $bindable(),
		verbs = [],
		nouns = []
	}: Props = $props();

	/** At the start, while the text can still be a verb, Tab completes verbs and nouns; after the verb, only nouns. */
	const wordsFor = (text: string) => {
		const t = text.toLowerCase();
		return verbs.some((v) => v.toLowerCase().startsWith(t)) ? [...verbs, ...nouns] : nouns;
	};

	// Tab completion. `base` is the typed text while Tab cycles through more than one match.
	let base: string | null = null;
	let index = 0;
	/** The text that the last Tab wrote. Another text (typed, or from a verb click) ends the cycle. */
	let shown = '';
	let matches = $state<string[]>([]);

	function keydown(event: KeyboardEvent) {
		if (event.key !== 'Tab' || event.altKey || event.ctrlKey || event.metaKey) return;
		if (base !== null && value !== shown) typed();
		if (base !== null) {
			// Tab again: the next match; Shift+Tab: the one before.
			event.preventDefault();
			index = (index + (event.shiftKey ? -1 : 1) + matches.length) % matches.length;
			value = shown = complete(base, wordsFor(base), index)?.text ?? value;
			return;
		}
		// Without text or without a match, Tab moves the focus as usual.
		if (event.shiftKey || !value.trim()) return;
		const c = complete(value, wordsFor(value));
		if (!c) {
			matches = [];
			return;
		}
		event.preventDefault();
		matches = c.matches;
		if (c.text !== value) value = shown = c.text;
		else {
			base = value;
			index = 0;
			value = shown = complete(base, wordsFor(base), 0)?.text ?? value;
		}
	}

	function typed() {
		base = null;
		matches = [];
	}

	function submit(event: SubmitEvent) {
		event.preventDefault();
		if (!value.trim()) return;
		oncommand(value);
		value = '';
		typed();
	}
</script>

<form class="prompt" onsubmit={submit}>
	<span class="caret" aria-hidden="true">&gt;</span>
	<TextInput
		id="command"
		label="Command"
		hideLabel
		bind:value
		bind:element={input}
		autocomplete="off"
		spellcheck="false"
		aria-describedby="command-matches"
		onkeydown={keydown}
		oninput={typed}
		{placeholder}
	/>
</form>
<p class="matches" id="command-matches" aria-live="polite">
	{#if matches.length > 1}{matches.join(' · ')}{/if}
</p>

<style>
	.matches {
		min-height: 1lh;
		margin: var(--space-1) 0 0 calc(1ch + var(--space-4));
		color: var(--color-text-muted);
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
	}
	.prompt {
		display: flex;
		align-items: center;
		gap: var(--space-4);
		max-width: var(--measure-prose);
		color: var(--color-text-muted);
		font-size: var(--font-size-lg);
	}
</style>

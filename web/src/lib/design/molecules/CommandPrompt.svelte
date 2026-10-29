<script lang="ts">
	import TextInput from '../atoms/TextInput.svelte';

	interface Props {
		/** Called with the typed command when the player pushes Enter. */
		oncommand: (text: string) => void;
		placeholder?: string;
		/** The input element, for focus from outside (for example a skip link). */
		input?: HTMLInputElement;
	}

	let {
		oncommand,
		placeholder = 'type a command, or use the verbs below',
		input = $bindable()
	}: Props = $props();

	let text = $state('');

	function submit(event: SubmitEvent) {
		event.preventDefault();
		if (!text.trim()) return;
		oncommand(text);
		text = '';
	}
</script>

<form class="prompt" onsubmit={submit}>
	<span class="caret" aria-hidden="true">&gt;</span>
	<TextInput
		id="command"
		label="Command"
		hideLabel
		bind:value={text}
		bind:element={input}
		autocomplete="off"
		spellcheck="false"
		{placeholder}
	/>
</form>

<style>
	.prompt {
		display: flex;
		align-items: center;
		gap: var(--space-4);
		max-width: var(--measure-prose);
		color: var(--color-text-muted);
		font-size: var(--font-size-lg);
	}
</style>

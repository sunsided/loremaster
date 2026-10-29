<script lang="ts">
	import TextInput from '../atoms/TextInput.svelte';

	interface Props {
		/** Called with the typed command when the player pushes Enter. */
		oncommand: (text: string) => void;
		placeholder?: string;
		/** The text in the prompt. Bind it to keep the prompt and other controls in sync. */
		value?: string;
		/** The input element, for focus from outside (for example a skip link). */
		input?: HTMLInputElement;
	}

	let {
		oncommand,
		placeholder = 'type a command, or use the verbs below',
		value = $bindable(''),
		input = $bindable()
	}: Props = $props();

	function submit(event: SubmitEvent) {
		event.preventDefault();
		if (!value.trim()) return;
		oncommand(value);
		value = '';
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

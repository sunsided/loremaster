<script lang="ts">
	import Button from '../atoms/Button.svelte';
	import Select from '../atoms/Select.svelte';
	import TextInput from '../atoms/TextInput.svelte';

	type Role = 'setup' | 'payoff' | 'mention';

	interface Props {
		/** The threads of the story. */
		threads: { id: string; name: string }[];
		/** Tie the text to a thread: a note with the thread and the role. */
		ontag: (beat: { thread: string; role: Role; text: string }) => void;
	}

	let { threads, ontag }: Props = $props();
	const uid = $props.id();
	// The first thread until the author picks one; a picked thread that goes away falls back too.
	let picked = $state('');
	const thread = $derived(threads.some((t) => t.id === picked) ? picked : (threads[0]?.id ?? ''));
	let role = $state<Role>('setup');
	let text = $state('');
	const roles: { value: Role; label: string }[] = [
		{ value: 'setup', label: 'setup' },
		{ value: 'mention', label: 'mention' },
		{ value: 'payoff', label: 'payoff' }
	];
	const name = $derived(threads.find((t) => t.id === thread)?.name ?? thread);
</script>

<!--
	Tie a text to a long-range thread: this text is a setup, a mention or a payoff of the thread.
	The tie is a note on the text, with the thread and the role. The explorer then checks the order.
-->
{#if threads.length}
	<form
		class="thread-tag"
		onsubmit={(e) => {
			e.preventDefault();
			ontag({ thread, role, text: text.trim() || `${role[0].toUpperCase()}${role.slice(1)} of ${name}.` });
			text = '';
		}}
	>
		<Select label="This text is a" options={roles} bind:value={role} />
		<Select
			label="of the thread"
			options={threads.map((t) => ({ value: t.id, label: t.name }))}
			value={thread}
			onchange={(e) => (picked = (e.currentTarget as HTMLSelectElement).value)}
		/>
		<TextInput id="{uid}-why" label="Why (optional)" bind:value={text} autocomplete="off" />
		<Button size="compact" type="submit">Tie to the thread</Button>
	</form>
{:else}
	<p class="none">The story has no threads yet. Make one in Atlas › Threads.</p>
{/if}

<style>
	.thread-tag {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-end;
		gap: var(--space-3) var(--space-4);
		font-size: var(--font-size-sm);
	}
	.none {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
</style>

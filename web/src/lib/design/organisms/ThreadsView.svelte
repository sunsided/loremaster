<script lang="ts">
	import Button from '../atoms/Button.svelte';
	import Select from '../atoms/Select.svelte';
	import TextInput from '../atoms/TextInput.svelte';
	import type { ThreadItem } from '../types.ts';

	type Status = ThreadItem['status'];

	interface Props {
		threads: ThreadItem[];
		/** True when the state explorer ran since the last change, so `reach` is known. */
		explored?: boolean;
		/** Change the status of a thread. */
		onstatus: (id: string, status: Status) => void;
		/** Make a new thread with this name. */
		oncreate: (name: string) => void;
		/** Go to the place of a beat. */
		onbeat: (key: string) => void;
		/** Run the state explorer, for the reach of the payoffs. */
		onexplore?: () => void;
	}

	let { threads, explored = false, onstatus, oncreate, onbeat, onexplore }: Props = $props();
	const uid = $props.id();
	let name = $state('');
	const statuses: { value: Status; label: string }[] = [
		{ value: 'open', label: 'open' },
		{ value: 'resolved', label: 'resolved' },
		{ value: 'dropped', label: 'dropped' }
	];
	const roleLabel = { setup: 'setup', payoff: 'payoff', mention: 'mention' } as const;
</script>

<!--
	The long-range threads of the story: promises to the reader, with their beats (setups, mentions, payoffs)
	from the notes all over the story. Warnings come from the checker and, after a run, from the state explorer.
-->
<section class="threads-view" aria-label="Threads">
	{#if threads.length}
		<ul class="threads">
			{#each threads as t (t.id)}
				<li class="thread" class:dropped={t.status === 'dropped'}>
					<div class="head">
						<h3>{t.name}</h3>
						<span class="counts">{t.setups} setup{t.setups === 1 ? '' : 's'} · {t.payoffs} payoff{t.payoffs === 1 ? '' : 's'}</span>
						<Select
							label="Status of {t.name}"
							hideLabel
							value={t.status}
							options={statuses}
							onchange={(e) => onstatus(t.id, (e.currentTarget as HTMLSelectElement).value as Status)}
						/>
					</div>
					{#if t.summary}<p class="summary">{t.summary}</p>{/if}
					{#each t.warnings as w (w)}<p class="warning">{w}</p>{/each}
					{#if t.beats.length}
						<ol class="beats">
							{#each t.beats as b (b.key)}
								<li>
									<span class="role {b.role ?? 'note'}">{b.role ? roleLabel[b.role] : b.todo ? 'todo' : 'note'}</span>
									<span class="text" class:done={b.todo && b.done}>{b.text}</span>
									{#if b.where}<button type="button" class="link" onclick={() => onbeat(b.key)}>{b.where}</button>{/if}
								</li>
							{/each}
						</ol>
					{:else}
						<p class="muted">No beats yet. Add a note with <code>thread: {t.id}</code> and a role to a room, a thing or a fragment.</p>
					{/if}
				</li>
			{/each}
		</ul>
	{:else}
		<p class="muted">No threads yet. A thread is a promise to the reader that resolves later, for example a hint that pays off at the end.</p>
	{/if}
	{#if onexplore && !explored && threads.some((t) => t.payoffs > 0)}
		<p class="muted">
			Run the state explorer to see if each payoff can show, and if it can show before its setup.
			<Button size="compact" variant="quiet" onclick={onexplore}>Explore</Button>
		</p>
	{/if}
	<form
		class="add"
		onsubmit={(e) => {
			e.preventDefault();
			if (!name.trim()) return;
			oncreate(name.trim());
			name = '';
		}}
	>
		<TextInput id="{uid}-name" label="New thread" bind:value={name} placeholder="A name, for example The lost key" autocomplete="off" />
		<Button size="compact" type="submit" disabled={!name.trim()}>+ Thread</Button>
	</form>
</section>

<style>
	.threads-view {
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
	}
	ul,
	ol {
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.threads {
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
	}
	.thread {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		padding-bottom: var(--space-5);
		border-bottom: var(--size-border) solid var(--color-border-subtle);
	}
	.thread.dropped {
		color: var(--color-text-muted);
	}
	.head {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-3) var(--space-5);
	}
	h3 {
		margin: 0;
		color: var(--color-text-strong);
		font-size: var(--font-size-md);
	}
	.counts,
	.muted {
		color: var(--color-text-muted);
	}
	p {
		margin: 0;
		line-height: var(--line-height-body);
	}
	.summary {
		color: var(--color-text);
	}
	.warning {
		color: var(--color-text-warning);
	}
	.beats {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.beats li {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: var(--space-2) var(--space-3);
	}
	.role {
		min-width: 6ch;
		color: var(--color-text-muted);
		text-transform: uppercase;
		font-size: var(--font-size-xs);
		letter-spacing: var(--letter-spacing-caps);
	}
	.role.setup,
	.role.payoff {
		color: var(--color-text-accent);
	}
	.text.done {
		text-decoration: line-through;
		color: var(--color-text-muted);
	}
	.link {
		min-height: var(--size-target-compact);
		padding: 0;
		border: 0;
		background: none;
		color: var(--color-text-accent);
		font: inherit;
		text-decoration: underline;
		cursor: pointer;
	}
	.add {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-end;
		gap: var(--space-3);
	}
</style>

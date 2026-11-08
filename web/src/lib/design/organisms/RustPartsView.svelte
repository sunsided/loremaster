<script lang="ts">
	import type { RustPart } from '#lib/engine/types.ts';
	import Button from '../atoms/Button.svelte';
	import Tag from '../atoms/Tag.svelte';

	interface Props {
		/** The parts made in Rust, by entity key, for example `room:shore`. */
		parts: Record<string, RustPart>;
		/** The file of each entity now. A part with a YAML file is ejected or overridden. */
		origins: Record<string, string>;
		/** True when the editor can write a YAML file for this part. */
		canOverride: (key: string) => boolean;
		/** True when the editor can eject this part. */
		canEject: (key: string) => boolean;
		/** Write the YAML file with `replace: true`. Returns a problem text, or null. */
		oneject: (key: string) => Promise<string | null>;
		/** Write the YAML file with `override:`. Returns a problem text, or null. */
		onoverride: (key: string) => Promise<string | null>;
		/** Delete the YAML file of an override. */
		ondelete: (file: string) => Promise<string | null>;
	}

	let { parts, origins, canOverride, canEject, oneject, onoverride, ondelete }: Props = $props();

	/** The part that waits for the confirmation of an override. */
	let confirming = $state<string | null>(null);
	let failure = $state<string | null>(null);

	async function run(action: () => Promise<string | null>) {
		failure = await action();
		confirming = null;
	}

	const entries = $derived(Object.entries(parts));
</script>

<!--
The parts of a story that are made in Rust. They are read-only here, and each shows its source line.
Override writes a YAML file that `assemble` compares with Rust, and asks first. Eject takes the part over for good.
-->
<section class="parts" aria-label="Parts made in Rust">
	{#if failure}<p class="failure" role="alert">{failure}</p>{/if}
	<ul>
		{#each entries as [key, part] (key)}
			{@const file = origins[key]}
			{@const yaml = part.overridden_by === null && file !== part.origin.file}
			<li>
				<div class="row">
					<code class="key">{key}</code>
					{#if part.overridden_by?.[1] === 'active'}
						<Tag tone="warning">override active</Tag>
					{:else if part.overridden_by?.[1] === 'no_longer_needed'}
						<Tag>same as Rust</Tag>
					{:else if yaml}
						<Tag>ejected to YAML</Tag>
					{:else}
						<Tag><svg aria-hidden="true" width="12" height="12" viewBox="0 0 16 16"><path fill="currentColor" d="M5 7V5a3 3 0 1 1 6 0v2h1v7H4V7zm1.5 0h3V5a1.5 1.5 0 1 0-3 0z" /></svg> Rust, read-only</Tag>
					{/if}
				</div>
				<p class="source">{part.origin.file}:{part.origin.line}:{part.origin.column}</p>
				{#if part.overridden_by}
					{@const [path, state] = part.overridden_by}
					<p class="status">
						{state === 'active' ? `Override in ${path}. Port it to the DSL with "loremaster codegen".` : `${path} is the same as Rust now, so it is no longer needed.`}
					</p>
					{#if state === 'no_longer_needed'}
						<Button size="compact" onclick={() => run(() => ondelete(path))}>Delete the override</Button>
					{/if}
				{:else if !yaml && canOverride(key)}
					{#if confirming === key}
						<div class="confirm" role="group" aria-label="Confirm the override">
							<p>Override this Rust part with a YAML file? The file keeps the Rust value, and a check fails when Rust changes later.</p>
							<Button size="compact" variant="primary" onclick={() => run(() => onoverride(key))}>Yes, override</Button>
							<Button size="compact" variant="quiet" onclick={() => (confirming = null)}>Cancel</Button>
						</div>
					{:else}
						<div class="actions">
							<Button size="compact" onclick={() => (confirming = key)}>Override</Button>
							{#if canEject(key)}
								<Button size="compact" variant="quiet" onclick={() => run(() => oneject(key))}>Eject to YAML</Button>
							{/if}
						</div>
					{/if}
				{/if}
			</li>
		{/each}
	</ul>
</section>

<style>
	.parts {
		font-family: var(--font-ui);
		font-size: var(--font-size-sm);
		color: var(--color-text);
	}
	ul {
		display: grid;
		gap: var(--space-4);
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.row,
	.actions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-3);
	}
	.source {
		margin: var(--space-1) 0;
		font-size: var(--font-size-xs);
		color: var(--color-text-muted);
		overflow-wrap: anywhere;
	}
	.status,
	.confirm p {
		margin: var(--space-2) 0;
	}
	.failure {
		color: var(--color-text-warning);
		white-space: pre-wrap;
	}
</style>

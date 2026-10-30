<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import StatusTray from './StatusTray.svelte';
	import AiStatus from './AiStatus.svelte';

	const jobs = [
		{ id: 'j2', task: 'imagine_exit', about: 'shore, north', status: 'running' as const, seconds: 12, cost: null, proposals: 0 },
		{ id: 'j1', task: 'review_room', about: 'shore', status: 'done' as const, seconds: 0, cost: 0.04, proposals: 0 }
	];

	const { Story } = defineMeta({
		title: 'Molecules/StatusTray',
		component: StatusTray,
		tags: ['autodocs'],
		parameters: {
			docs: {
				description: {
					component: `A chip on the status line that opens a small tray under it. The tray is for things of the whole session, not of the current room. The editor uses it for the AI jobs (AiStatus).

- The chip is a disclosure button with \`aria-expanded\`. Its name is its text, then \`hint\` for screen readers. The tray is a group with the title as its name.
- When the tray opens, it gets the focus. **Escape** closes it and moves the focus back to the chip. A click outside closes it.
- The tray is fixed under the chip and stays inside the screen, also on a phone.
- \`meta\` is a short fact at the right of the title, for example the cost so far.`
				}
			}
		}
	});
</script>

<Story name="AI jobs" asChild>
	<div style="padding: 8px; background: var(--color-status-bg); color: var(--color-status-text); font-family: var(--font-ui); font-size: var(--font-size-sm); font-weight: var(--font-weight-strong);">
		<StatusTray hint="Show the AI jobs." title="AI · loremaster serve" meta="$0.04">
			{#snippet chip()}✦ drafting… 12 s{/snippet}
			<AiStatus {jobs} oncancel={fn()} onproposals={fn()} />
		</StatusTray>
	</div>
</Story>
<Story name="Open" asChild>
	<div style="min-height: 320px; padding: 8px; background: var(--color-status-bg); color: var(--color-status-text); font-family: var(--font-ui); font-size: var(--font-size-sm); font-weight: var(--font-weight-strong);">
		<StatusTray hint="Show the AI jobs." title="AI · loremaster serve" meta="$0.04" open>
			{#snippet chip()}✦ 2 proposals{/snippet}
			<AiStatus {jobs} differ={['rooms/shore.yaml']} oncancel={fn()} onlink={fn()} onproposals={fn()} />
		</StatusTray>
	</div>
</Story>

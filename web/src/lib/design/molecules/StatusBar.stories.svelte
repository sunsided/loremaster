<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import Button from '../atoms/Button.svelte';
	import StatusBar from './StatusBar.svelte';

	const { Story } = defineMeta({
		title: 'Molecules/StatusBar',
		component: StatusBar,
		tags: ['autodocs'],
		args: { items: ['turn 2', '20:40', 'cottage'] },
		parameters: {
			docs: {
				description: {
					component: `The status bar shows short facts about the game, such as the turn, the time and the room.

The facts are a list. Controls go in the \`actions\` snippet at the end of the bar. On narrow screens the actions go to a new line.`
				}
			}
		}
	});
</script>

<Story name="Facts only" />
<Story name="With actions" asChild>
	<StatusBar items={['turn 2', '20:40', 'cottage']}>
		{#snippet actions()}
			<Button size="compact" variant="quiet">Restart</Button>
		{/snippet}
	</StatusBar>
</Story>

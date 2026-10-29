<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { rules } from '../fixtures.ts';
	import RulesView from './RulesView.svelte';

	const { Story } = defineMeta({
		title: 'Organisms/RulesView',
		component: RulesView,
		tags: ['autodocs'],
		args: { rules },
		parameters: {
			docs: {
				description: {
					component: `The rules view shows all rules of the story as RuleCards, with their state in the current game.

The cards are in a grid. The number of columns changes with the width of the screen. On a phone there is one column.
The summary line gives the number of rules and how many fired.`
				}
			}
		}
	});
</script>

<Story name="Lighthouse rules" />
<Story name="No rules" args={{ rules: [] }} />

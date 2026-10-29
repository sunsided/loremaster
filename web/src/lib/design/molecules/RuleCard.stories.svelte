<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { rules } from '../fixtures.ts';
	import RuleCard from './RuleCard.svelte';

	const { Story } = defineMeta({
		title: 'Molecules/RuleCard',
		component: RuleCard,
		tags: ['autodocs'],
		args: { rule: rules[0] },
		parameters: {
			docs: {
				description: {
					component: `A rule card shows one rule as three lines: **When** (the event), **If** (the condition) and **Then** (the effects, in order).

The header shows the rule id and its state:
- "fired" or "not fired yet" in this game. A fired rule has an accent line on the left.
- "once" when the rule fires only one time.
- "instead of the default" when the rule replaces the normal response of the verb.

The condition shows "true now" or "false now" in the current game state.
The card is an \`article\` with the rule id as its heading.`
				}
			}
		}
	});
</script>

<Story name="Fired" />
<Story name="Instead of the default" args={{ rule: rules[1] }} />
<Story name="No condition" args={{ rule: rules[2] }} />

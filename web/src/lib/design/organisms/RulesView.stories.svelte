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
					component: `The rules view shows the rules of the story in the inspector. It is made for big games with hundreds of rules.

1. **Show** selects which rules are in the list:
   - "Here and now" (the default) shows only the rules that matter at the player's position and time: rules about this room or a room next to it, actions with things in reach, time rules in the next hour, rules that check every turn, and rules that fired in the last turn. The runtime gives the reason, and the list shows it.
   - "Fired" shows the rules that fired in this game.
   - "All" shows every rule.
2. **Filter** keeps only the rules that contain all the typed words, in the id, event, condition or effects.
3. The rules are in groups by event: Rooms, Actions, Time, Every turn and Start. Each group can close.
4. Each rule is one line. Open the line to see the full RuleCard.

The command palette can show one rule: the view then selects "All", opens the rule and moves focus to its line.
All controls are native elements (\`details\`, \`summary\`, radio buttons, a text input), so they work with the keyboard and screen readers.`
				}
			}
		}
	});
</script>

<Story name="Here and now" />
<Story name="Show one rule" args={{ reveal: 'first_climb' }} />
<Story name="No rules" args={{ rules: [] }} />

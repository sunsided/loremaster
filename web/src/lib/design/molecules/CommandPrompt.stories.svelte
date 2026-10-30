<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import CommandPrompt from './CommandPrompt.svelte';

	const { Story } = defineMeta({
		title: 'Molecules/CommandPrompt',
		component: CommandPrompt,
		tags: ['autodocs'],
		args: { oncommand: fn() },
		parameters: {
			docs: {
				description: {
					component: `The command prompt is where the player types a command, such as "open drawer".

Push Enter to send the command. The prompt then becomes empty. An empty command does nothing.
The input has the id \`command\`. The skip link of the page moves focus to it.
The label "Command" is hidden visually but screen readers announce it.

Bind \`value\` to keep the prompt and the verb bar in sync. The page does this: typing marks the verb and the things on the verb bar, and a click on the verb bar writes into the prompt.

**Tab completes** the last word from \`verbs\` (at the start of a command) and \`nouns\` (the things in reach and carried, and the exits). One match completes the word and adds a space. More matches complete to their common start and show under the prompt (a polite live region); Tab again goes to the next match, Shift+Tab to the one before. In an empty prompt, or when nothing fits, Tab moves the focus as usual, so the keyboard is never trapped.`
				}
			}
		}
	});
</script>

<Story name="Default" />
<Story name="Custom placeholder" args={{ placeholder: 'what now?' }} />
<Story name="With a partial command" args={{ value: 'look lighth' }} />
<Story
	name="Tab completion"
	args={{ value: 'take o', verbs: ['take', 'pick up', 'open', 'look'], nouns: ['oil lamp', 'oak door', 'north'] }}
/>

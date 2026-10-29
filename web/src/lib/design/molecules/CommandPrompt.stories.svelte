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
The label "Command" is hidden visually but screen readers announce it.`
				}
			}
		}
	});
</script>

<Story name="Default" />
<Story name="Custom placeholder" args={{ placeholder: 'what now?' }} />

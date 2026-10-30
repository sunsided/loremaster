<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import Button from '../atoms/Button.svelte';
	import NewStoryDialog from './NewStoryDialog.svelte';

	const { Story } = defineMeta({
		title: 'Organisms/NewStoryDialog',
		component: NewStoryDialog,
		tags: ['autodocs'],
		parameters: {
			docs: {
				description: {
					component: `The new story dialog starts a story from zero.

The author gives a title. The idea is optional:
- Without an idea, the story has one room with a placeholder text.
- With an idea, the AI drafts two or three connected rooms as one proposal. The author accepts or rejects it.

When \`loremaster serve\` does not run, the idea field still shows, with the reason in its help text. Then the dialog makes the empty story.

It is a modal \`dialog\`. When it opens, focus goes to the title. Escape closes it. Enter in the title creates the story.`
				},
				story: { inline: false, iframeHeight: 560 }
			}
		}
	});
</script>

<script lang="ts">
	let open = $state(false);
	let made = $state('');
</script>

<Story name="With the AI" asChild>
	<Button aria-haspopup="dialog" onclick={() => (open = true)}>New story</Button>
	<p>{made}</p>
	<NewStoryDialog bind:open oncreate={(s) => (made = `Created “${s.title}”${s.idea ? ` from the idea “${s.idea}”` : ''}.`)} />
</Story>

<Story name="Without the AI" asChild>
	<Button aria-haspopup="dialog" onclick={() => (open = true)}>New story</Button>
	<p>{made}</p>
	<NewStoryDialog
		bind:open
		aiUnavailable="Start `loremaster serve` for this story to use the AI."
		oncreate={(s) => (made = `Created “${s.title}”.`)}
	/>
</Story>

<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import ThreadTag from './ThreadTag.svelte';

	const { Story } = defineMeta({
		title: 'Molecules/ThreadTag',
		component: ThreadTag,
		tags: ['autodocs'],
		args: {
			threads: [
				{ id: 'lost_key', name: 'The lost key' },
				{ id: 'the_marigold', name: 'The Marigold' }
			],
			ontag: fn()
		},
		parameters: {
			docs: {
				description: {
					component: `Ties a text to a long-range thread: the text is a setup, a mention or a payoff.

The tie is a note on the fragment: \`{ text: "…", thread: lost_key, role: setup }\`. Without a "why", the note text is for example "Setup of The lost key.". The state explorer then checks that a payoff can show, and not before a setup.

The fragment panel shows this form under "Notes and threads of this fragment".`
				}
			}
		}
	});
</script>

<Story name="Default" />
<Story name="No threads" args={{ threads: [] }} />

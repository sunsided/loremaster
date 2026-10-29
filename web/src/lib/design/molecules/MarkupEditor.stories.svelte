<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import { writerTools } from '../fixtures.ts';
	import MarkupEditor from './MarkupEditor.svelte';

	const { Story } = defineMeta({
		title: 'Molecules/MarkupEditor',
		component: MarkupEditor,
		tags: ['autodocs'],
		args: {
			label: 'Text',
			value: 'East, the lighthouse rises {night: black against the stars | pale against the {time.phase} sky}. *Marigold*.',
			tokenize: (t: string) => writerTools.tokenize(t, false),
			onsubmit: fn(),
			oncancel: fn()
		},
		parameters: {
			docs: {
				description: {
					component: `The markup editor is a text editor (CodeMirror 6) for fragment text with inline markup, or for a condition.

- The highlights come from a tokenizer function. In the app it is the runtime tokenizer in WebAssembly, so the highlights always agree with the game. The stories use a small tokenizer in JavaScript.
- Braces, separators and operators are muted. Names are in the accent colour. Emphasis (\`*word*\`) is italic.
- \`problem\` puts a wavy line under a range of the text. The parent shows the problem message and connects it with \`describedby\`.
- Enter calls \`onsubmit\`. In a multi-line editor, Shift+Enter adds a line break. Escape calls \`oncancel\`.
- \`singleLine\` is for conditions: the text stays on one line, and the font is the interface font.

The editor has the accessible name from \`label\`, and \`aria-multiline\`.`
				}
			}
		}
	});
</script>

<Story name="Fragment text" />
<Story
	name="With a problem"
	args={{ value: 'The sun is a {lamp.lot ? bright : red} coal.', problem: { from: 14, to: 22, message: 'unknown name `lamp.lot`' } }}
/>
<Story
	name="Condition"
	args={{ value: 'time.phase == night and not beacon_lit', singleLine: true, label: 'Condition', tokenize: (t: string) => writerTools.tokenize(t, true) }}
/>
<Story name="Empty, with placeholder" args={{ value: '', placeholder: 'Write what the player reads…' }} />

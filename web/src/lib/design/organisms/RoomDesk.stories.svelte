<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { fn } from 'storybook/test';
	import { blackbeardOutline, map, rules } from '../fixtures.ts';
	import RoomDesk from './RoomDesk.svelte';
	import DialogueOutline from './DialogueOutline.svelte';
	import NotesList from './NotesList.svelte';
	import AiHere from '../molecules/AiHere.svelte';

	const switches = [
		{ name: 'beacon_lit', value: 'false', options: ['false', 'true'] },
		{ name: 'time.phase', value: 'dusk', options: ['night', 'dawn', 'day', 'dusk'] }
	];
	const threads = [
		{ thread: 'marigold', name: 'The Marigold', role: 'setup' as const },
		{ thread: 'hals-lamp', name: "Hal's lamp", role: 'setup' as const },
		{ thread: 'hals-lamp', name: "Hal's lamp", role: 'missing' as const }
	];
	const notes = [
		{ key: 'a', text: "Hal's boots by the door?", todo: true, done: false },
		{ key: 'b', text: 'The bell buoy must sound like the one in the stairwell.', todo: false, done: false }
	];

	const { Story } = defineMeta({
		title: 'Organisms/RoomDesk',
		component: RoomDesk,
		tags: ['autodocs'],
		parameters: {
			docs: {
				description: {
					component: `The room desk is the side panel in write mode. It holds what the author can change in the current room. Things about the whole story are in the Atlas. Things about one line of text are in the margin.

1. **The head**: the room name, its file and its level. Under it, one line from the last explorer run: "reachable" or "not reached", and the number of lines that never show. If the explorer did not run yet, "Explore ↗" runs it. The line does not start a new run by itself.
2. **Depends on**: the switches with the preview (StateSwitches), as in the Here panel.
3. **Notes and threads**: the beats of threads in this room as chips, then the notes and TODOs (the \`notes\` snippet, usually NotesList). A chip opens Atlas › Threads. "no payoff yet" is a warning: an open thread has a setup here and no payoff.
4. **Dialogue**: the dialogues of the things in the room (the \`dialogue\` snippet, usually one DialogueOutline for each thing). It shows only when a thing here has a dialogue.
5. **✦ AI here**: the AI help for this room (the \`ai\` snippet, usually AiHere). It shows only while the AI server runs.
6. **Rules here**: the rules that matter here and now.
7. **Around you**: the small map, last. Set \`map\` to null when the big map shows on the left.

With \`tabs\`, for the bottom sheet on a phone, the sections are three tabs: Room, Notes and AI. The arrow keys move between the tabs.

The desk is a region with the name "Room desk: " and the room name.`
				}
			}
		}
	});
</script>

{#snippet notesList()}
	<NotesList {notes} ontoggle={fn()} onadd={fn()} onremove={fn()} />
{/snippet}
{#snippet dialogue()}
	<DialogueOutline outline={blackbeardOutline} name="Blackbeard" file="dialogues/blackbeard.yaml" />
{/snippet}
{#snippet aiHere()}
	<AiHere onreview={fn()} onproposals={fn()} findings={2} proposals={1} />
{/snippet}

<Story name="Desk" asChild>
	<RoomDesk
		room="The Shore"
		file="rooms/shore.yaml"
		level={0}
		reach={{ reached: true, states: 412, truncated: false, neverShown: ['#3'] }}
		onexplore={fn()}
		{switches}
		texts={8}
		onpreview={fn()}
		onreset={fn()}
		onapply={fn()}
		notes={notesList}
		notesMeta="2 · 1 TODO"
		{threads}
		onthread={fn()}
		ai={aiHere}
		rules={rules.filter((r) => r.relevant)}
		ruleCount={rules.length}
		onrule={fn()}
		{map}
		onopen={fn()}
	/>
</Story>
<Story name="Not explored, no AI" asChild>
	<RoomDesk
		room="The Shore"
		file="rooms/shore.yaml"
		onexplore={fn()}
		{switches}
		texts={8}
		onpreview={fn()}
		onreset={fn()}
		onapply={fn()}
		notes={notesList}
		onthread={fn()}
		rules={[]}
		ruleCount={rules.length}
		onrule={fn()}
		onopen={fn()}
	/>
</Story>
<Story name="Not reached" asChild>
	<RoomDesk
		room="The Vault"
		file="rooms/vault.yaml"
		reach={{ reached: false, states: 412, truncated: false, neverShown: [] }}
		onexplore={fn()}
		switches={[]}
		onpreview={fn()}
		onreset={fn()}
		onapply={fn()}
		notes={notesList}
		onthread={fn()}
		rules={[]}
		ruleCount={rules.length}
		onrule={fn()}
		onopen={fn()}
	/>
</Story>
<Story name="With a dialogue" asChild>
	<RoomDesk
		room="The Cantina"
		file="rooms/cantina.yaml"
		reach={{ reached: true, states: 64, truncated: false, neverShown: [] }}
		onexplore={fn()}
		switches={[{ name: 'knows_blackbeard_name', value: 'false', options: ['false', 'true'] }]}
		texts={3}
		onpreview={fn()}
		onreset={fn()}
		onapply={fn()}
		notes={notesList}
		onthread={fn()}
		{dialogue}
		dialogueMeta="Blackbeard · 4 nodes"
		rules={[]}
		ruleCount={0}
		onrule={fn()}
		onopen={fn()}
	/>
</Story>
<Story name="Phone tabs" asChild>
	<div style="max-width: 390px;">
		<RoomDesk
			room="The Shore"
			file="rooms/shore.yaml"
			reach={{ reached: true, states: 412, truncated: false, neverShown: [] }}
			onexplore={fn()}
			{switches}
			texts={8}
			onpreview={fn()}
			onreset={fn()}
			onapply={fn()}
			notes={notesList}
			{threads}
			onthread={fn()}
			ai={aiHere}
			rules={rules.filter((r) => r.relevant)}
			ruleCount={rules.length}
			onrule={fn()}
			onopen={fn()}
			tabs
		/>
	</div>
</Story>

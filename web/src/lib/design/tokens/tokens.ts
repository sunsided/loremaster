// Token names with their purpose, for the Storybook token pages.
// tokens.css is the source of the values. Keep this list in sync with it.

export interface TokenInfo {
	name: string;
	use: string;
}

export const colorTokens: TokenInfo[] = [
	{ name: '--color-surface-outer', use: 'Background around the app.' },
	{ name: '--color-surface', use: 'Background of the player screen.' },
	{ name: '--color-surface-raised', use: 'Background of panels and dialogs.' },
	{ name: '--color-surface-control', use: 'Background of buttons.' },
	{ name: '--color-text', use: 'Body text.' },
	{ name: '--color-text-strong', use: 'Text that must stand out, such as links in prose.' },
	{ name: '--color-text-muted', use: 'Secondary text, such as labels and echoed commands.' },
	{ name: '--color-text-accent', use: 'Headings and the app title.' },
	{ name: '--color-text-warning', use: 'Fragment numbers and reasons why text is hidden.' },
	{ name: '--color-text-on-accent', use: 'Text on an accent background.' },
	{ name: '--color-accent', use: 'Selected controls and the text cursor.' },
	{ name: '--color-accent-tint', use: 'A faint accent fill, for a selected item that must not draw the eye.' },
	{ name: '--color-status-bg', use: 'Background of the status line (inverse video): the accent of the theme.' },
	{ name: '--color-status-text', use: 'Text on the status line.' },
	{ name: '--color-ghost', use: 'AI proposals: ghost text and proposal cards. A cooler tone than the accent.' },
	{ name: '--color-ghost-tint', use: 'A faint ghost fill behind ghost text.' },
	{ name: '--color-warning', use: 'Edges of highlighted fragments.' },
	{ name: '--color-border-subtle', use: 'Dividers. Decoration only.' },
	{ name: '--color-border-control', use: 'Edge of a control. Contrast 3:1 or more.' },
	{ name: '--color-focus', use: 'Focus ring. Contrast 3:1 or more.' },
	{ name: '--color-scrollbar-thumb', use: 'Scrollbar handle.' },
	{ name: '--color-scrollbar-thumb-hover', use: 'Scrollbar handle under the pointer (WebKit only).' }
];

export const spaceTokens: TokenInfo[] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10].map((n) => ({
	name: `--space-${n}`,
	use: n <= 4 ? 'Gaps inside a component.' : n <= 7 ? 'Gaps between components.' : 'Padding of regions.'
}));

export const typeTokens: TokenInfo[] = [
	{ name: '--font-size-xs', use: 'Fragment numbers.' },
	{ name: '--font-size-sm', use: 'Meta text and notes.' },
	{ name: '--font-size-md', use: 'Interface text. This is the default.' },
	{ name: '--font-size-lg', use: 'Command input and exits.' },
	{ name: '--font-size-xl', use: 'Large interface text.' },
	{ name: '--font-size-heading', use: 'Dialog headings.' },
	{ name: '--font-size-prose', use: 'Story text. It changes with the viewport width.' }
];

export const sizeTokens: TokenInfo[] = [
	{ name: '--size-target', use: 'Minimum height of a primary control (44px).' },
	{ name: '--size-target-compact', use: 'Minimum height of a secondary control (32px).' },
	{ name: '--measure-prose', use: 'Maximum line length of story text.' },
	{ name: '--width-panel', use: 'Maximum width of the side panel (Here panel, fragment panel).' },
	{ name: '--width-margin', use: 'Width of the margin notes next to the text in write mode.' },
	{ name: '--width-dialog', use: 'Maximum width of a dialog.' },
	{ name: '--width-map', use: 'Maximum width of the map on ultrawide screens.' },
	{ name: '--width-player-max', use: 'Maximum width of the player on ultrawide screens.' },
	{ name: '--width-content-max', use: 'Maximum width of all content. Wider screens center it.' }
];

export const motionTokens: TokenInfo[] = [
	{ name: '--duration-fast', use: 'Hover and focus changes.' },
	{ name: '--duration-base', use: 'Line reveal with subtle effects.' },
	{ name: '--duration-slow', use: 'Line reveal with full effects.' },
	{ name: '--duration-line-step', use: 'Delay between two revealed lines.' },
	{ name: '--duration-glitch', use: 'Length of the glitch on new text.' },
	{ name: '--motion-sheet', use: 'Height change of the bottom sheet on phones (off with reduced motion).' }
];

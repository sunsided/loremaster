// Which help notes the user collapsed. The choice stays in browser storage when it is available.

const KEY = 'lantern.help.collapsed';

function read(): string[] {
	try {
		return JSON.parse(localStorage.getItem(KEY) ?? '[]');
	} catch {
		return [];
	}
}

class HelpState {
	collapsed = $state<string[]>(typeof localStorage === 'undefined' ? [] : read());

	isCollapsed(id: string): boolean {
		return this.collapsed.includes(id);
	}

	set(id: string, collapsed: boolean) {
		this.collapsed = collapsed ? [...new Set([...this.collapsed, id])] : this.collapsed.filter((x) => x !== id);
		try {
			localStorage.setItem(KEY, JSON.stringify(this.collapsed));
		} catch {
			// Storage is not available: the choice lasts for this visit only.
		}
	}
}

export const help = new HelpState();

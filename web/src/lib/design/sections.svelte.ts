// Which panel sections the user folded. The choice stays in browser storage when it is available.

import '#lib/migrate.ts';

const KEY = 'loremaster.sections.closed';

function read(): string[] {
	try {
		return JSON.parse(localStorage.getItem(KEY) ?? '[]');
	} catch {
		return [];
	}
}

class SectionState {
	closed = $state<string[]>(typeof localStorage === 'undefined' ? [] : read());

	isClosed(id: string): boolean {
		return this.closed.includes(id);
	}

	set(id: string, closed: boolean) {
		this.closed = closed ? [...new Set([...this.closed, id])] : this.closed.filter((x) => x !== id);
		try {
			localStorage.setItem(KEY, JSON.stringify(this.closed));
		} catch {
			// Storage is not available: the choice lasts for this visit only.
		}
	}
}

export const sections = new SectionState();

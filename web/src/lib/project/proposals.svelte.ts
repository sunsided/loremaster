// Proposals from `lantern serve` (docs/ai-tools.md): changes that an AI assistant suggests.
// The editor reads them from the local server, shows them as ghost text, and applies an accepted
// proposal to its own working copy. Then it tells the server, so the server drops the proposal.

import { keepLines } from './yaml-edit.ts';
import type { Workspace } from './workspace.svelte.ts';

/** The address of `lantern serve`. */
export const SERVE = 'http://127.0.0.1:4710';

export interface ProposalFragment {
	id: string | null;
	variants: string[];
	when: string | null;
	mode: string;
	layer: string;
	replaces: boolean;
}

export type ProposalChange =
	| { kind: 'added'; entity: string; file: string; after: unknown }
	| { kind: 'removed'; entity: string; file: string; before: unknown }
	| { kind: 'field'; entity: string; file: string; field: string; before: unknown; after: unknown }
	| {
			kind: 'fragment';
			entity: string;
			file: string;
			owner: string;
			field: string[];
			fragment_id?: string | null;
			before_index?: number | null;
			after_index?: number | null;
			before_fragment?: ProposalFragment | null;
			after_fragment?: ProposalFragment | null;
	  };

export interface ProposalView {
	id: string;
	summary: string;
	ok: boolean;
	stale: boolean;
	check: { severity: string; message: string; file: string }[];
	new_problems: { severity: string; message: string; file: string }[];
	changes: ProposalChange[];
	rooms: { room: string; texts: { text: string; combinations: number; warnings: string[] }[] }[];
	files: { path: string; base_sha: string | null; before: string | null; after: string | null }[];
}

async function sha256(text: string): Promise<string> {
	const bytes = new TextEncoder().encode(text);
	const hash = await crypto.subtle.digest('SHA-256', bytes);
	return [...new Uint8Array(hash)].map((b) => b.toString(16).padStart(2, '0')).join('');
}

/** The link to `lantern serve`: the open proposals, read again from time to time. */
export class AiLink {
	/** True while the server answers. */
	reachable = $state(false);
	proposals = $state<ProposalView[]>([]);
	#timer: ReturnType<typeof setTimeout> | null = null;
	#stopped = false;

	/** Read the proposals now, then again after a pause: often while the server answers, rarely when it does not. */
	start() {
		this.#stopped = false;
		void this.#poll();
	}

	stop() {
		this.#stopped = true;
		if (this.#timer) clearTimeout(this.#timer);
	}

	async #poll() {
		try {
			const res = await fetch(`${SERVE}/proposals`);
			if (!res.ok) throw new Error(String(res.status));
			this.proposals = (await res.json()) as ProposalView[];
			this.reachable = true;
		} catch {
			this.reachable = false;
			this.proposals = [];
		}
		if (!this.#stopped) this.#timer = setTimeout(() => this.#poll(), this.reachable ? 4000 : 20000);
	}

	async #post(path: string, body: unknown) {
		const res = await fetch(`${SERVE}${path}`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify(body)
		});
		if (!res.ok) throw new Error(`${res.status} ${await res.text()}`);
	}

	/**
	 * Apply a proposal to the working copy of the editor, then tell the server.
	 * Returns a problem text when it cannot: a file changed since the proposal, or the story does not build.
	 */
	async accept(view: ProposalView, workspace: Workspace): Promise<string | null> {
		if (!view.ok) return 'The proposal has problems. Fix or reject it.';
		const changes: { path: string; text: string }[] = [];
		for (const f of view.files) {
			const now = workspace.project.file(f.path)?.text ?? null;
			if (f.base_sha !== null && (now === null || (await sha256(now)) !== f.base_sha))
				return `The file ${f.path} changed after the proposal was made. Reject it, or ask for a new one.`;
			if (f.base_sha === null && now !== null) return `The file ${f.path} exists now. The proposal wanted to make it.`;
			if (f.after === null) return `The proposal deletes ${f.path}. The editor cannot delete files yet; accept it in the server.`;
			// Keep the lines that the proposal did not change byte for byte.
			changes.push({ path: f.path, text: now === null ? f.after : keepLines(now, f.after) });
		}
		const problems = await workspace.changeMany(changes);
		if (problems?.length) return problems.map((p) => p.message).join(' ');
		try {
			await this.#post(`/proposals/${view.id}/accept`, { applied: true });
		} catch {
			// The change is in the files; the server drops the proposal at its next look, or the author rejects it there.
		}
		this.proposals = this.proposals.filter((p) => p.id !== view.id);
		return null;
	}

	async reject(view: ProposalView, note?: string): Promise<string | null> {
		try {
			await this.#post(`/proposals/${view.id}/reject`, note ? { note } : {});
			this.proposals = this.proposals.filter((p) => p.id !== view.id);
			return null;
		} catch (e) {
			return String(e);
		}
	}
}

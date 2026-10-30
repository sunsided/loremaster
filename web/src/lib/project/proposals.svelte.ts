// Proposals from `lantern serve` (docs/ai-tools.md): changes that an AI assistant suggests.
// The editor reads them from the local server, shows them as ghost text, and applies an accepted
// proposal to its own working copy. Then it tells the server, so the server drops the proposal.

import type { ReviewFinding } from '#lib/design/types.ts';
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

/** A job of `lantern serve`: one task that Claude Code does, for example `imagine_exit`. */
export interface JobView {
	id: string;
	task: string;
	args: Record<string, unknown>;
	status: 'running' | 'done' | 'failed' | 'cancelled';
	events: { kind: string; text: string }[];
	result: string | null;
	proposals: string[];
	cost_usd: number | null;
	model: string | null;
	/** When the editor started the job, for the elapsed time. The server does not send it. */
	started?: number;
}

/** Whether the files that the server reads are the files of the editor. */
export interface SyncState {
	/** True when the checked files are the same. */
	same: boolean;
	/** Files that differ, or that only one side has. */
	differ: string[];
	/** The number of files that the check compared by content. */
	checked: number;
}

/**
 * The findings of a `review_room` job, from its reply: one line each, in the form
 * `- high | room:shore #2 | the problem | the fix`. Lines in another form are left out.
 */
export function parseFindings(job: string, reply: string): ReviewFinding[] {
	const out: ReviewFinding[] = [];
	for (const line of reply.split('\n')) {
		const m = line.match(/^\s*(?:[-*]|\d+\.)?\s*\**\s*(high|medium|low)\s*\**\s*\|\s*`?([^|#`]+?)`?\s*(?:#\s*(\d+))?\s*\|\s*(.+?)\s*\|\s*(.+?)\s*$/i);
		if (!m) continue;
		out.push({
			key: `${job}#${out.length}`,
			severity: m[1].toLowerCase() as ReviewFinding['severity'],
			owner: m[2].trim(),
			fragment: m[3] === undefined ? null : Number(m[3]),
			problem: m[4],
			fix: m[5]
		});
	}
	return out;
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
	jobs = $state<JobView[]>([]);
	/** The last comparison of the files of the server and of the editor, or null before the first one. */
	sync = $state<SyncState | null>(null);
	#timer: ReturnType<typeof setTimeout> | null = null;
	#started = new Map<string, number>();
	#stopped = true;
	#polling = false;

	/** Read the proposals now, then again after a pause: often while the server answers, rarely when it does not. */
	start() {
		if (!this.#stopped) return;
		this.#stopped = false;
		void this.#poll();
	}

	stop() {
		this.#stopped = true;
		if (this.#timer) clearTimeout(this.#timer);
	}

	/** Read now, unless a read runs already. Then schedule the next read; only one timer is pending. */
	async #poll() {
		if (this.#polling) return;
		this.#polling = true;
		if (this.#timer) clearTimeout(this.#timer);
		this.#timer = null;
		try {
			const res = await fetch(`${SERVE}/proposals`);
			if (!res.ok) throw new Error(String(res.status));
			this.proposals = (await res.json()) as ProposalView[];
			this.reachable = true;
		} catch {
			this.reachable = false;
			this.proposals = [];
			this.jobs = [];
		}
		if (this.reachable) {
			// A failed jobs read keeps the proposals and the last known jobs.
			try {
				const jobs = await fetch(`${SERVE}/jobs`);
				if (jobs.ok)
					this.jobs = ((await jobs.json()) as JobView[]).map((j) => ({ ...j, started: this.#started.get(j.id) ?? Date.now() }));
			} catch {
				// Read the jobs again at the next poll.
			}
		}
		this.#polling = false;
		const running = this.jobs.some((j) => j.status === 'running');
		if (this.#timer) clearTimeout(this.#timer);
		if (!this.#stopped) this.#timer = setTimeout(() => this.#poll(), !this.reachable ? 20000 : running ? 1500 : 4000);
	}

	/** Call one MCP tool of the server and give its text result. */
	async #tool(name: string, args: Record<string, unknown>): Promise<string> {
		const res = await fetch(`${SERVE}/mcp`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'tools/call', params: { name, arguments: args } })
		});
		const body = await res.json();
		if (body.error || body.result?.isError) throw new Error(body.error?.message ?? body.result?.content?.[0]?.text ?? 'tool error');
		const text = body.result?.content?.[0]?.text;
		if (typeof text !== 'string') throw new Error(`The tool ${name} gave no text result.`);
		return text;
	}

	/**
	 * Compare the files that the server reads with the files of the editor. The paths are compared always;
	 * the contents of the edited files, and of all files in a small story (up to 80 files).
	 */
	async compare(files: { path: string; text: string }[], edited: Set<string>): Promise<SyncState | null> {
		if (!this.reachable) return null;
		try {
			const paths: string[] = JSON.parse(await this.#tool('list_files', {}));
			const mine = new Map(files.map((f) => [f.path, f.text]));
			const differ = [...paths.filter((p) => !mine.has(p)), ...[...mine.keys()].filter((p) => !paths.includes(p))];
			const toRead = paths.filter((p) => mine.has(p) && (files.length <= 80 || edited.has(p)));
			for (const path of toRead) {
				const { text } = JSON.parse(await this.#tool('read_file', { path })) as { text: string };
				if (text !== mine.get(path)) differ.push(path);
			}
			this.sync = { same: differ.length === 0, differ, checked: toRead.length };
		} catch {
			this.sync = null;
		}
		return this.sync;
	}

	/** Start a job, for example `imagine_exit` with `{ room, direction }`. `state` is the save state of the editor. */
	async startJob(task: string, args: Record<string, unknown>, state?: unknown): Promise<string | null> {
		try {
			const res = await fetch(`${SERVE}/jobs`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ task, args, ...(state ? { state } : {}) })
			});
			if (!res.ok) return `${res.status} ${await res.text()}`;
			const { id } = (await res.json()) as { id: string };
			this.#started.set(id, Date.now());
			this.jobs = [...this.jobs, { id, task, args, status: 'running', events: [], result: null, proposals: [], cost_usd: null, model: null, started: Date.now() }];
			if (this.#timer) clearTimeout(this.#timer);
			this.#timer = setTimeout(() => this.#poll(), 1500);
			return null;
		} catch (e) {
			return String(e);
		}
	}

	/**
	 * Start a new story on the server from an idea: `POST /bootstrap`. The server writes the new story with
	 * this title, the same files as the editor's new story, and starts a `seed_story` job.
	 * Returns a problem text, for example when the server's folder has a story already.
	 */
	async bootstrap(title: string, idea: string): Promise<string | null> {
		try {
			const res = await fetch(`${SERVE}/bootstrap`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ title, idea })
			});
			if (res.status === 409)
				return 'The AI server has a story already. Start `lantern serve --bootstrap` for a new story, then try again.';
			if (!res.ok) return `${res.status} ${await res.text()}`;
			const { id } = (await res.json()) as { id: string };
			this.#started.set(id, Date.now());
			const args = { idea };
			this.jobs = [...this.jobs, { id, task: 'seed_story', args, status: 'running', events: [], result: null, proposals: [], cost_usd: null, model: null, started: Date.now() }];
			if (this.#timer) clearTimeout(this.#timer);
			this.#timer = setTimeout(() => this.#poll(), 1500);
			return null;
		} catch (e) {
			return String(e);
		}
	}

	/** True for a proposal that a `seed_story` job made: the first rooms of a new story. */
	isSeed(id: string): boolean {
		return this.jobs.some((j) => j.task === 'seed_story' && j.proposals.includes(id));
	}

	async cancelJob(id: string) {
		try {
			await this.#post(`/jobs/${id}/cancel`, {});
		} catch {
			// The job ended meanwhile.
		}
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
		try {
			return await this.#accept(view, workspace, this.isSeed(view.id));
		} catch (e) {
			return `The editor could not apply the proposal: ${e instanceof Error ? e.message : String(e)}`;
		}
	}

	/**
	 * With `serverWrites`, the server writes the files into its own folder too, so that the next jobs see
	 * the new rooms. The seed of a new story does this: the server's folder is the editor's story.
	 */
	async #accept(view: ProposalView, workspace: Workspace, serverWrites = false): Promise<string | null> {
		const changes: { path: string; text: string | null }[] = [];
		for (const f of view.files) {
			const now = workspace.project.file(f.path)?.text ?? null;
			if (f.base_sha !== null && (now === null || (await sha256(now)) !== f.base_sha))
				return `The file ${f.path} changed after the proposal was made. Reject it, or ask for a new one.`;
			if (f.base_sha === null && now !== null) return `The file ${f.path} exists now. The proposal wanted to make it.`;
			if (f.after === null) {
				changes.push({ path: f.path, text: null });
				continue;
			}
			// Keep the lines that the proposal did not change byte for byte.
			changes.push({ path: f.path, text: now === null ? f.after : keepLines(now, f.after) });
		}
		const problems = await workspace.changeMany(changes);
		if (problems?.length) return problems.map((p) => p.message).join(' ');
		try {
			await this.#post(`/proposals/${view.id}/accept`, serverWrites ? {} : { applied: true });
		} catch {
			// The change is in the files; the server drops the proposal at its next look, or the author rejects it there.
		}
		this.proposals = this.proposals.filter((p) => p.id !== view.id);
		return null;
	}

	/** Reject a proposal. The note tells the assistant why; `imagine_exit` does not offer the idea again. */
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

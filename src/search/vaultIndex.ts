import { App, TFile } from "obsidian";
import {
	VaultFile,
	searchTodos,
	searchRequires,
	headingCheck,
	collectTags,
	SearchResult,
	PersonResults,
	HeadingMismatch,
} from "../../task-management-bindings/pkg/task_management_bindings";

/**
 * Reads vault markdown files and runs them through the wasm search bindings.
 *
 * wasm has no filesystem access, so every call here reads file content via
 * the Obsidian Vault API and hands it to Rust as plain data. Todos/requires/
 * heading-check are recomputed on demand (cheap enough to run once per modal
 * open); the tag list is different — it backs the `#`-suggest popup, so it
 * can't afford a full vault rescan per keystroke, and is cached here across
 * launches via serialize()/restore().
 */
export class VaultIndex {
	private app: App;
	private tagCache: string[] = [];

	constructor(app: App) {
		this.app = app;
	}

	/** Plain data for the plugin to fold into its single persisted data.json blob. */
	serialize(): string[] {
		return this.tagCache;
	}

	restore(tagCache: string[] | undefined) {
		this.tagCache = tagCache ?? [];
	}

	private async readAllFiles(): Promise<VaultFile[]> {
		const files = this.app.vault.getMarkdownFiles();
		const contents = await Promise.all(files.map((f) => this.app.vault.cachedRead(f)));
		return files.map((f, i) => new VaultFile(f.path, contents[i]!));
	}

	async searchTodos(include: string[] = [], any: string[] = [], exclude: string[] = []): Promise<SearchResult[]> {
		const files = await this.readAllFiles();
		return searchTodos(files, include, any, exclude);
	}

	async searchRequires(): Promise<PersonResults[]> {
		const files = await this.readAllFiles();
		return searchRequires(files);
	}

	async headingMismatches(): Promise<HeadingMismatch[]> {
		const files = await this.readAllFiles();
		return headingCheck(files);
	}

	tags(): string[] {
		return this.tagCache;
	}

	async ensureTagCache(): Promise<void> {
		if (this.tagCache.length === 0) {
			await this.rescanTags();
		}
	}

	async rescanTags(): Promise<void> {
		const files = await this.readAllFiles();
		this.tagCache = collectTags(files);
	}

	/** Re-derive tags for a single changed file and merge them into the cache. */
	async updateTagsFor(file: TFile): Promise<void> {
		const content = await this.app.vault.cachedRead(file);
		const fileTags = collectTags([new VaultFile(file.path, content)]);
		const merged = new Set([...this.tagCache, ...fileTags]);
		this.tagCache = [...merged].sort();
	}
}

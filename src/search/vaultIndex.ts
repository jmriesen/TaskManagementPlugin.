import { App } from "obsidian";
import {
	VaultFile,
	searchTodos,
	searchRequires,
	headingCheck,
	SearchResult,
	PersonResults,
	HeadingMismatch,
} from "../../task-management-bindings/pkg/task_management_bindings";

/**
 * Reads vault markdown files and runs them through the wasm search bindings.
 *
 * wasm has no filesystem access, so every call here reads file content via
 * the Obsidian Vault API and hands it to Rust as plain data.
 */
export class VaultIndex {
	private app: App;

	constructor(app: App) {
		this.app = app;
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
}

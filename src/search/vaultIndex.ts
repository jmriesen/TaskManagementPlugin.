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

	/**
	 * TEMPORARY DIAGNOSTIC: parses one file at a time (instead of the whole
	 * vault in a single wasm call) so the console shows exactly which file was
	 * being processed if/when it hangs. Remove once the hang is root-caused.
	 */
	async diagnoseHang(): Promise<void> {
		const files = await this.readAllFiles();
		console.log(`diagnoseHang: starting, ${files.length} files`);
		for (const file of files) {
			const path = file.path;
			console.log(`diagnoseHang: parsing ${path} (${file.content.length} chars)`);
			searchTodos([file], [], [], []);
			console.log(`diagnoseHang: done ${path}`);
		}
		console.log("diagnoseHang: finished all files, no hang");
	}

	/**
	 * TEMPORARY DIAGNOSTIC: bisects the *bulk* call (all files in one wasm
	 * invocation, which is what every real search command uses) by trying it
	 * with increasing batch sizes, logging before/after each attempt. Since
	 * per-file calls are known-good, this narrows down whether the hang is a
	 * function of how many files are processed within a single wasm call
	 * (no JS boundary crossing in between) rather than total work done.
	 * Remove once the hang is root-caused.
	 */
	async diagnoseBulkHang(): Promise<void> {
		const files = await this.readAllFiles();
		console.log(`diagnoseBulkHang: starting, ${files.length} files total`);
		const sizes = [1, 2, 5, 10, 20, 50, 100, 150, files.length];
		for (const n of sizes) {
			if (n > files.length) continue;
			const batch = files.slice(0, n);
			console.log(`diagnoseBulkHang: trying batch of ${n} files...`);
			const start = Date.now();
			searchTodos(batch, [], [], []);
			console.log(`diagnoseBulkHang: batch of ${n} succeeded in ${Date.now() - start}ms`);
		}
		console.log("diagnoseBulkHang: finished all batch sizes, no hang");
	}
}

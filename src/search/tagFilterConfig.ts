import { App, Notice, TAbstractFile, TFile } from "obsidian";
import { parseTagFilters, NamedTagFilter } from "../../task-management-bindings/pkg/task_management_bindings";

/**
 * Vault-root file holding a `name -> TagFilter` map, shared (eventually) with
 * the nvim plugin's equivalent config. See core::tag_filters for the TOML
 * format. Mirrors the nvim workflow of keeping this in one frequently
 * hand-edited file rather than plugin settings.
 *
 * Uses a `.md` extension (with the TOML wrapped in a ```toml fenced code
 * block) rather than a bare `.toml` file because Obsidian mobile won't open
 * non-markdown files for editing.
 */
export const TAG_FILTERS_FILE_PATH = "task-filters.md";

const TOML_BLOCK = /```toml\n([\s\S]*?)```/;

function extractToml(content: string): string {
	const match = content.match(TOML_BLOCK);
	return match ? match[1]! : content;
}

export class TagFilterConfig {
	constructor(
		private app: App,
		private onFiltersChanged: (filters: NamedTagFilter[]) => void,
	) {}

	isConfigFile(file: TAbstractFile): file is TFile {
		return file instanceof TFile && file.path === TAG_FILTERS_FILE_PATH;
	}

	async load(): Promise<void> {
		const file = this.app.vault.getAbstractFileByPath(TAG_FILTERS_FILE_PATH);
		if (!(file instanceof TFile)) {
			this.onFiltersChanged([]);
			return;
		}
		await this.reloadFrom(file);
	}

	async reloadFrom(file: TFile): Promise<void> {
		const content = await this.app.vault.cachedRead(file);
		try {
			this.onFiltersChanged(parseTagFilters(extractToml(content)));
		} catch (e) {
			new Notice(`Failed to parse ${TAG_FILTERS_FILE_PATH}:\n${e}`);
		}
	}
}

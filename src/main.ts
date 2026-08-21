import { Plugin, TAbstractFile, TFile } from 'obsidian';
import { DEFAULT_SETTINGS, TaskManagementSettings, SettingTab } from "./settings";
import { registerExcerptProcessor } from "./excerpt/processor";
import { VaultIndex } from "./search/vaultIndex";
import { TodosModal } from "./search/todosModal";
import { RequiresModal } from "./search/requiresModal";
import { HeadingCheckModal } from "./search/headingCheckModal";
import { TagSuggest } from "./search/tagSuggest";
import wasmBinary from '../task-management-bindings/pkg/task_management_bindings_bg.wasm';
import * as wasmNamespace from '../task-management-bindings/pkg/task_management_bindings';

interface PersistedData {
	settings: TaskManagementSettings;
	tagCache: string[];
}

export default class TaskManagment extends Plugin {
	settings: TaskManagementSettings;
	vaultIndex: VaultIndex;

	async onload() {
		const wasmModule = await import('../task-management-bindings/pkg/task_management_bindings.js')
		wasmModule.initSync({ module: wasmBinary });
		this.vaultIndex = new VaultIndex(this.app);
		await this.loadPersistedData();
		this.addSettingTab(new SettingTab(this.app, this));
		registerExcerptProcessor(this);

		await this.vaultIndex.ensureTagCache();
		await this.savePersistedData();

		this.registerEvent(this.app.vault.on("modify", (file) => this.onFileChanged(file)));
		this.registerEvent(this.app.vault.on("create", (file) => this.onFileChanged(file)));
		this.registerEditorSuggest(new TagSuggest(this.app, this.vaultIndex));

		this.addCommand({
			id: "rescan-vault-tags",
			name: "Rescan vault tags",
			callback: async () => {
				await this.vaultIndex.rescanTags();
				await this.savePersistedData();
			},
		});

		this.addCommand({
			id: "search-todos",
			name: "Search todos",
			callback: async () => {
				new TodosModal(this.app, await this.vaultIndex.searchTodos()).open();
			},
		});

		this.addCommand({
			id: "search-next-actions",
			name: "Search next actions",
			callback: async () => {
				new TodosModal(this.app, await this.vaultIndex.searchTodos(["#NextAction"])).open();
			},
		});

		this.addCommand({
			id: "search-requires",
			name: "Search requires",
			callback: async () => {
				new RequiresModal(this.app, await this.vaultIndex.searchRequires()).open();
			},
		});

		this.addCommand({
			id: "heading-check",
			name: "Heading check",
			callback: async () => {
				new HeadingCheckModal(this.app, await this.vaultIndex.headingMismatches()).open();
			},
		});
	}

	private async onFileChanged(file: TAbstractFile) {
		if (!(file instanceof TFile) || file.extension !== "md") return;
		await this.vaultIndex.updateTagsFor(file);
		await this.savePersistedData();
	}

	onunload() {
	}

	async loadPersistedData() {
		const data = (await this.loadData() ?? {}) as Partial<PersistedData>;
		this.settings = Object.assign({}, DEFAULT_SETTINGS, data.settings);
		this.vaultIndex.restore(data.tagCache);
	}

	async saveSettings() {
		await this.savePersistedData();
	}

	private async savePersistedData() {
		const data: PersistedData = {
			settings: this.settings,
			tagCache: this.vaultIndex.serialize(),
		};
		await this.saveData(data);
	}
}

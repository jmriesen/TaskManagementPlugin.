import { Plugin } from 'obsidian';
import { DEFAULT_SETTINGS, TaskManagementSettings, SettingTab } from "./settings";
import { registerExcerptProcessor } from "./excerpt/processor";
import { VaultIndex } from "./search/vaultIndex";
import { TodosModal } from "./search/todosModal";
import { RequiresModal } from "./search/requiresModal";
import { HeadingCheckModal } from "./search/headingCheckModal";
import wasmBinary from '../task-management-bindings/pkg/task_management_bindings_bg.wasm';
import * as wasmNamespace from '../task-management-bindings/pkg/task_management_bindings';

export default class TaskManagment extends Plugin {
	settings: TaskManagementSettings;
	vaultIndex: VaultIndex;

	async onload() {
		const wasmModule = await import('../task-management-bindings/pkg/task_management_bindings.js')
		wasmModule.initSync({ module: wasmBinary });
		this.vaultIndex = new VaultIndex(this.app);
		await this.loadSettings();
		this.addSettingTab(new SettingTab(this.app, this));
		registerExcerptProcessor(this);

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

	onunload() {
	}

	async loadSettings() {
		this.settings = Object.assign({}, DEFAULT_SETTINGS, await this.loadData() as Partial<TaskManagementSettings>);
	}

	async saveSettings() {
		await this.saveData(this.settings);
	}
}

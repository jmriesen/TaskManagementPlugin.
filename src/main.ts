import { Plugin } from 'obsidian';
import { DEFAULT_SETTINGS, TaskManagementSettings, SettingTab } from "./settings";
import { registerExcerptProcessor } from "./excerpt/processor";
import { VaultIndex } from "./search/vaultIndex";
import { TodosModal } from "./search/todosModal";
import { RequiresModal } from "./search/requiresModal";
import { HeadingCheckModal } from "./search/headingCheckModal";
import { TagFilterConfig } from "./search/tagFilterConfig";
import { NamedTagFilter } from "../task-management-bindings/pkg/task_management_bindings";
import wasmBinary from '../task-management-bindings/pkg/task_management_bindings_bg.wasm';
import * as wasmNamespace from '../task-management-bindings/pkg/task_management_bindings';

export default class TaskManagement extends Plugin {
	settings: TaskManagementSettings;
	vaultIndex: VaultIndex;
	tagFilterConfig: TagFilterConfig;
	private filterCommandIds: string[] = [];

	async onload() {
		const wasmModule = await import('../task-management-bindings/pkg/task_management_bindings.js')
		wasmModule.initSync({ module: wasmBinary });
		this.vaultIndex = new VaultIndex(this.app);
		await this.loadSettings();
		this.addSettingTab(new SettingTab(this.app, this));
		registerExcerptProcessor(this);

		this.tagFilterConfig = new TagFilterConfig(this.app, (filters) => this.registerFilterCommands(filters));
		await this.tagFilterConfig.load();
		this.registerEvent(this.app.vault.on("modify", (file) => {
			if (this.tagFilterConfig.isConfigFile(file)) {
				this.tagFilterConfig.reloadFrom(file);
			}
		}));
		this.registerEvent(this.app.vault.on("create", (file) => {
			if (this.tagFilterConfig.isConfigFile(file)) {
				this.tagFilterConfig.reloadFrom(file);
			}
		}));

		this.addCommand({
			id: "search-todos",
			name: "Search todos",
			callback: async () => {
				new TodosModal(this.app, await this.vaultIndex.searchTodos()).open();
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

	/** Re-registers one command per named filter, replacing whatever was registered from the previous version of the file. */
	private registerFilterCommands(namedFilters: NamedTagFilter[]) {
		for (const id of this.filterCommandIds) {
			this.removeCommand(id);
		}
		this.filterCommandIds = namedFilters.map(({ name, filter }) => {
			const id = `search-filter-${name.toLowerCase().replace(/\s+/g, "-")}`;
			this.addCommand({
				id,
				name: `Search: ${name}`,
				callback: async () => {
					const results = await this.vaultIndex.searchTodos(filter.include, filter.any, filter.exclude);
					new TodosModal(this.app, results).open();
				},
			});
			return id;
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

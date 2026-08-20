import { Plugin, TAbstractFile, TFile } from 'obsidian';
import { DEFAULT_SETTINGS, TaskManagementSettings, SettingTab } from "./settings";
import { registerExcerptProcessor } from "./excerpt/processor";
import { VaultIndex } from "./search/vaultIndex";
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
		wasmModule.initSync(wasmBinary);
		this.vaultIndex = new VaultIndex(this.app);
		await this.loadPersistedData();
		this.addSettingTab(new SettingTab(this.app, this));
		registerExcerptProcessor(this);

		await this.vaultIndex.ensureTagCache();
		await this.savePersistedData();

		this.registerEvent(this.app.vault.on("modify", (file) => this.onFileChanged(file)));
		this.registerEvent(this.app.vault.on("create", (file) => this.onFileChanged(file)));

		this.addCommand({
			id: "rescan-vault-tags",
			name: "Rescan vault tags",
			callback: async () => {
				await this.vaultIndex.rescanTags();
				await this.savePersistedData();
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

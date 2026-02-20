import { Plugin } from 'obsidian';
import { DEFAULT_SETTINGS, TaskManagementSettings, SettingTab } from "./settings";
import { registerExcerptProcessor } from "./excerpt/processor";
import wasmBinary from '../task-management-bindings/pkg/task_management_bindings_bg.wasm';
import * as wasmNamespace from '../task-management-bindings/pkg/task_management_bindings';

export default class TaskManagment extends Plugin {
	settings: TaskManagementSettings;

	async onload() {
		const wasmModule = await import('../task-management-bindings/pkg/task_management_bindings.js')
		wasmModule.initSync(wasmBinary);
		await this.loadSettings();
		this.addSettingTab(new SettingTab(this.app, this));
		registerExcerptProcessor(this);
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

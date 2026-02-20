import { Notice, Plugin } from 'obsidian';
import { DEFAULT_SETTINGS, TaskManagementSettings, SettingTab } from "./settings";
import { registerExcerptProcessor } from "./excerpt/processor";
import wasmBinary from '../task-management-core/pkg/task_management_core_bg.wasm';
import * as wasmNamespace from '../task-management-core/pkg/task_management_core';

export default class TaskManagment extends Plugin {
	settings: TaskManagementSettings;

	async onload() {
		const wasmModule = await import('../task-management-core/pkg/task_management_core.js')
		wasmModule.initSync(wasmBinary);
		await this.loadSettings();
		this.addSettingTab(new SettingTab(this.app, this));
		registerExcerptProcessor(this);
		this.addRibbonIcon("cpu", "Run WASM foo()", async () => {
			const result = wasmNamespace.foo();
			new Notice(`foo() returned: ${result}`);
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

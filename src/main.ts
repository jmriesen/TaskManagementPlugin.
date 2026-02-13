import { Plugin } from 'obsidian';
import { DEFAULT_SETTINGS, TaskManagementSettings, SettingTab } from "./settings";
import { registerExcerptProcessor } from "./excerpt/processor";

export default class TaskManagment extends Plugin {
	settings: TaskManagementSettings;

	async onload() {
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

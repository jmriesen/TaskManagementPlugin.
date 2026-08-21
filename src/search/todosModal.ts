import { App, FuzzySuggestModal } from "obsidian";
import { SearchResult as TaskSearchResult } from "../../task-management-bindings/pkg/task_management_bindings";
import { openAtLine } from "./openResult";

/** Mirrors the nvim `search_tasks` Telescope picker. */
export class TodosModal extends FuzzySuggestModal<TaskSearchResult> {
	constructor(app: App, private results: TaskSearchResult[]) {
		super(app);
		this.setPlaceholder("Search todos…");
	}

	getItems(): TaskSearchResult[] {
		return this.results;
	}

	getItemText(item: TaskSearchResult): string {
		return item.ordinal;
	}

	async onChooseItem(item: TaskSearchResult) {
		await openAtLine(this.app, item.path, item.lnum);
	}
}

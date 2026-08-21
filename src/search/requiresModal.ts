import { App, FuzzySuggestModal } from "obsidian";
import { PersonResults, SearchResult as TaskSearchResult } from "../../task-management-bindings/pkg/task_management_bindings";
import { openAtLine } from "./openResult";

/** Second step of the nvim `requires` picker: one person's tasks. */
class PersonTasksModal extends FuzzySuggestModal<TaskSearchResult> {
	constructor(app: App, private person: string, private tasks: TaskSearchResult[]) {
		super(app);
		this.setPlaceholder(`Requires — ${person}`);
	}

	getItems(): TaskSearchResult[] {
		return this.tasks;
	}

	getItemText(item: TaskSearchResult): string {
		return item.ordinal;
	}

	async onChooseItem(item: TaskSearchResult) {
		await openAtLine(this.app, item.path, item.lnum);
	}
}

/** Mirrors the nvim `requires` Telescope picker: people, drilling down into their tasks. */
export class RequiresModal extends FuzzySuggestModal<PersonResults> {
	constructor(app: App, private people: PersonResults[]) {
		super(app);
		this.setPlaceholder("Requires — people…");
	}

	getItems(): PersonResults[] {
		return [...this.people].sort((a, b) => a.person.localeCompare(b.person));
	}

	getItemText(item: PersonResults): string {
		return `${item.person} (${item.tasks.length})`;
	}

	onChooseItem(item: PersonResults) {
		new PersonTasksModal(this.app, item.person, item.tasks).open();
	}
}

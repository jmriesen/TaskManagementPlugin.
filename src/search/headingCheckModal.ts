import { App, FuzzySuggestModal } from "obsidian";
import { HeadingMismatch } from "../../task-management-bindings/pkg/task_management_bindings";
import { openAtLine } from "./openResult";

/** Mirrors the nvim `heading-check` Telescope picker. */
export class HeadingCheckModal extends FuzzySuggestModal<HeadingMismatch> {
	constructor(app: App, private results: HeadingMismatch[]) {
		super(app);
		this.setPlaceholder("Heading mismatches…");
	}

	getItems(): HeadingMismatch[] {
		return this.results;
	}

	getItemText(item: HeadingMismatch): string {
		return `${item.filename}  →  ${item.heading ?? "(no h1 heading)"}`;
	}

	async onChooseItem(item: HeadingMismatch) {
		await openAtLine(this.app, item.path, 1);
	}
}

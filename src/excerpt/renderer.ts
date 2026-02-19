import { App, Component, MarkdownRenderer, setIcon } from "obsidian";
import { ExcerptError, ExcerptResult } from "./types";

export async function renderExcerpt(
	app: App,
	container: HTMLElement,
	result: ExcerptResult,
	component: Component,
	onEdit?: () => void
): Promise<void> {
	const wrapper = container.createDiv({ cls: "excerpt-embed" });

	if (onEdit) {
		const editBtn = wrapper.createEl("button", {
			cls: "excerpt-embed-edit-btn",
			attr: { "aria-label": "Edit excerpt source" },
		});
		setIcon(editBtn, "pencil");
		editBtn.addEventListener("click", (e) => {
			e.stopPropagation();
			onEdit();
		});
	}

	await MarkdownRenderer.render(
		app,
		result.content,
		wrapper,
		result.sourcePath,
		component
	);
}

export function renderExcerptError(
	container: HTMLElement,
	error: ExcerptError
): void {
	const wrapper = container.createDiv({ cls: "excerpt-embed excerpt-embed-error" });
	wrapper.createSpan({ text: error.message });
}

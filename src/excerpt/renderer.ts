import { App, Component, MarkdownRenderer } from "obsidian";
import { ExcerptError, ExcerptResult } from "./types";

export async function renderExcerpt(
	app: App,
	container: HTMLElement,
	result: ExcerptResult,
	component: Component
): Promise<void> {
	const wrapper = container.createDiv({ cls: "excerpt-embed" });
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

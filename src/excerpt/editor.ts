import { App, MarkdownView, TFile } from "obsidian";
import { ExcerptDirective } from "./types";

export async function openExcerptInEditor(
	app: App,
	directive: ExcerptDirective
): Promise<void> {
	const abstractFile = app.vault.getAbstractFileByPath(directive.filePath);
	if (!abstractFile || !(abstractFile instanceof TFile)) {
		return;
	}

	const leaf = app.workspace.getLeaf("tab");
	await leaf.openFile(abstractFile);

	const view = leaf.view;
	if (!(view instanceof MarkdownView)) {
		return;
	}

	const editor = view.editor;
	const fromLine = directive.lineStart - 1;
	const toLine = directive.lineEnd - 1;

	editor.setSelection(
		{ line: fromLine, ch: 0 },
		{ line: toLine, ch: editor.getLine(toLine).length }
	);
	editor.scrollIntoView(
		{ from: { line: fromLine, ch: 0 }, to: { line: toLine, ch: 0 } },
		true
	);
}

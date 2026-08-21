import { App, TFile } from "obsidian";

/** Open a file and place the cursor on a 1-indexed line, as returned by the search bindings. */
export async function openAtLine(app: App, path: string, lnum: number) {
	const file = app.vault.getAbstractFileByPath(path);
	if (!(file instanceof TFile)) return;
	const leaf = app.workspace.getLeaf(false);
	await leaf.openFile(file, { eState: { line: Math.max(0, lnum - 1) } });
}

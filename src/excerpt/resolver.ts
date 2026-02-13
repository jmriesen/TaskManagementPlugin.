import { App, TFile } from "obsidian";
import { ExcerptDirective, ExcerptError, ExcerptResult } from "./types";

export async function resolveFileContent(
	app: App,
	directive: ExcerptDirective
): Promise<ExcerptResult> {
	return resolveVaultPath(app, directive);
}

async function resolveVaultPath(
	app: App,
	directive: ExcerptDirective
): Promise<ExcerptResult> {
	const abstractFile = app.vault.getAbstractFileByPath(directive.filePath);
	if (!abstractFile || !(abstractFile instanceof TFile)) {
		throw new ExcerptError(`File not found in vault: ${directive.filePath}`);
	}
	const content = await app.vault.cachedRead(abstractFile);
	return { content, sourcePath: directive.filePath };
}

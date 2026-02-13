import { App, TFile } from "obsidian";
import { readFile } from "fs/promises";
import { ExcerptDirective, ExcerptError, ExcerptResult } from "./types";

export async function resolveFileContent(
	app: App,
	directive: ExcerptDirective
): Promise<ExcerptResult> {
	if (directive.isAbsolutePath) {
		return resolveAbsolutePath(directive);
	}
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

async function resolveAbsolutePath(
	directive: ExcerptDirective
): Promise<ExcerptResult> {
	try {
		const content = await readFile(directive.filePath, "utf-8");
		return { content, sourcePath: directive.filePath };
	} catch {
		throw new ExcerptError(`Failed to read file: ${directive.filePath}`);
	}
}

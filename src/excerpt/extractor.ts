import { App, } from "obsidian";
import { ExcerptDirective, ExcerptError, ExcerptResult } from "./types";

export function extractContent(
	app: App,
	fullContent: string,
	sourcePath: string,
	directive: ExcerptDirective
): ExcerptResult {
	const lines = fullContent.split("\n");
	const start = directive.lineStart - 1; // Convert 1-based to 0-based
	const end = directive.lineEnd;

	if (start < 0 || end > lines.length) {
		throw new ExcerptError(
			`Line range ${directive.lineStart}-${directive.lineEnd} out of bounds (file has ${lines.length} lines)`
		);
	}

	const content = lines.slice(start, end).join("\n");
	return { content, sourcePath };
}

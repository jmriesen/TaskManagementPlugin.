import { App, TFile } from "obsidian";
import { ExcerptDirective, ExcerptError, ExcerptResult } from "./types";

export function extractContent(
	app: App,
	fullContent: string,
	sourcePath: string,
	directive: ExcerptDirective
): ExcerptResult {
	if (directive.heading) {
		return extractByHeading(app, fullContent, sourcePath, directive);
	}
	if (directive.lineStart != null && directive.lineEnd != null) {
		return extractByLineRange(fullContent, sourcePath, directive);
	}
	return { content: fullContent, sourcePath };
}

function extractByHeading(
	app: App,
	fullContent: string,
	sourcePath: string,
	directive: ExcerptDirective
): ExcerptResult {
	if (!directive.isAbsolutePath) {
		const file = app.vault.getAbstractFileByPath(sourcePath);
		if (file instanceof TFile) {
			const cache = app.metadataCache.getFileCache(file);
			if (cache?.headings) {
				const result = extractHeadingFromCache(fullContent, cache.headings, directive.heading!);
				if (result != null) {
					return { content: result, sourcePath };
				}
			}
		}
	}

	// Regex fallback for absolute paths or when cache is unavailable
	const result = extractHeadingByRegex(fullContent, directive.heading!);
	if (result == null) {
		throw new ExcerptError(`Heading "${directive.heading}" not found in ${sourcePath}`);
	}
	return { content: result, sourcePath };
}

interface HeadingInfo {
	heading: string;
	level: number;
	position: { start: { line: number } };
}

function extractHeadingFromCache(
	fullContent: string,
	headings: HeadingInfo[],
	targetHeading: string
): string | null {
	const lines = fullContent.split("\n");
	const targetIndex = headings.findIndex(
		(h) => h.heading.toLowerCase() === targetHeading.toLowerCase()
	);
	if (targetIndex === -1) {
		return null;
	}

	const target = headings[targetIndex]!;
	const startLine = target.position.start.line;

	// Find the next heading at the same or higher level
	let endLine = lines.length;
	for (let i = targetIndex + 1; i < headings.length; i++) {
		if (headings[i]!.level <= target.level) {
			endLine = headings[i]!.position.start.line;
			break;
		}
	}

	return lines.slice(startLine, endLine).join("\n").trimEnd();
}

function extractHeadingByRegex(
	fullContent: string,
	targetHeading: string
): string | null {
	const lines = fullContent.split("\n");
	let startLine = -1;
	let headingLevel = 0;

	for (let i = 0; i < lines.length; i++) {
		const match = lines[i]!.match(/^(#{1,6})\s+(.+)$/);
		if (match && match[2]!.trim().toLowerCase() === targetHeading.toLowerCase()) {
			startLine = i;
			headingLevel = match[1]!.length;
			break;
		}
	}

	if (startLine === -1) {
		return null;
	}

	let endLine = lines.length;
	for (let i = startLine + 1; i < lines.length; i++) {
		const match = lines[i]!.match(/^(#{1,6})\s+/);
		if (match && match[1]!.length <= headingLevel) {
			endLine = i;
			break;
		}
	}

	return lines.slice(startLine, endLine).join("\n").trimEnd();
}

function extractByLineRange(
	fullContent: string,
	sourcePath: string,
	directive: ExcerptDirective
): ExcerptResult {
	const lines = fullContent.split("\n");
	const start = directive.lineStart! - 1; // Convert 1-based to 0-based
	const end = directive.lineEnd!;

	if (start < 0 || end > lines.length) {
		throw new ExcerptError(
			`Line range ${directive.lineStart}-${directive.lineEnd} out of bounds (file has ${lines.length} lines)`
		);
	}

	const content = lines.slice(start, end).join("\n");
	return { content, sourcePath };
}

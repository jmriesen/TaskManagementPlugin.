import { MarkdownRenderChild, Plugin } from "obsidian";
import { parseExcerptDirective } from "./parser";
import { resolveFileContent } from "./resolver";
import { extractContent } from "./extractor";
import { renderExcerpt, renderExcerptError } from "./renderer";
import { ExcerptError } from "./types";

export function registerExcerptProcessor(plugin: Plugin): void {
	plugin.registerMarkdownCodeBlockProcessor("excerpt", async (source, el, ctx) => {
		const child = new MarkdownRenderChild(el);
		ctx.addChild(child);

		try {
			const directive = parseExcerptDirective(source);
			const resolved = await resolveFileContent(plugin.app, directive);
			const result = extractContent(
				plugin.app,
				resolved.content,
				resolved.sourcePath,
				directive
			);
			await renderExcerpt(plugin.app, el, result, child);
		} catch (error) {
			if (error instanceof ExcerptError) {
				renderExcerptError(el, error);
			} else {
				renderExcerptError(el, new ExcerptError(
					`Unexpected error: ${error instanceof Error ? error.message : String(error)}`
				));
			}
		}
	});
}

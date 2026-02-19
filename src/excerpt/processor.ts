import { MarkdownRenderChild, Plugin, TFile } from "obsidian";
import { parseExcerptDirective } from "./parser";
import { resolveFileContent } from "./resolver";
import { extractContent } from "./extractor";
import { renderExcerpt, renderExcerptError } from "./renderer";
import { openExcerptInEditor } from "./editor";
import { ExcerptError } from "./types";

export function registerExcerptProcessor(plugin: Plugin): void {
	plugin.registerMarkdownCodeBlockProcessor("excerpt", async (source, el, ctx) => {
		const child = new MarkdownRenderChild(el);
		ctx.addChild(child);

		try {
			const directive = parseExcerptDirective(source);

			const renderContent = async () => {
				el.empty();
				const resolved = await resolveFileContent(plugin.app, directive);
				const result = extractContent(
					plugin.app,
					resolved.content,
					resolved.sourcePath,
					directive
				);
				await renderExcerpt(plugin.app, el, result, child, () => {
					openExcerptInEditor(plugin.app, directive);
				});
			};

			await renderContent();

			const onModify = async (file: TFile) => {
				if (file.path === directive.filePath) {
					await renderContent();
				}
			};
			const ref = plugin.app.vault.on("modify", onModify);
			child.register(() => plugin.app.vault.offref(ref));
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

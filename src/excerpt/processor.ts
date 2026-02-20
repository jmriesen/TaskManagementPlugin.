import { MarkdownRenderChild, Plugin, TFile } from "obsidian";
import { parseExcerptDirective, extractContent } from "../../task-management-core/pkg/task_management_core";
import { resolveFileContent } from "./resolver";
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
				const content = extractContent(resolved.content, directive.lineStart, directive.lineEnd);
				const result = { content, sourcePath: resolved.sourcePath };
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
			const msg = error instanceof Error ? error.message : String(error);
			renderExcerptError(el, new ExcerptError(msg));
		}
	});
}

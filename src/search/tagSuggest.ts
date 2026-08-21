import {
	App,
	Editor,
	EditorPosition,
	EditorSuggest,
	EditorSuggestContext,
	EditorSuggestTriggerInfo,
	TFile,
} from "obsidian";
import { VaultIndex } from "./vaultIndex";

/**
 * Mirrors the nvim `add_tags` picker: triggers on `#` and inserts the chosen
 * tag from the cached vault tag list, but as an inline EditorSuggest instead
 * of a separate command+modal — better mobile UX since it reuses the
 * on-screen keyboard flow the user is already in.
 */
export class TagSuggest extends EditorSuggest<string> {
	private vaultIndex: VaultIndex;

	constructor(app: App, vaultIndex: VaultIndex) {
		super(app);
		this.vaultIndex = vaultIndex;
	}

	onTrigger(cursor: EditorPosition, editor: Editor, _file: TFile | null): EditorSuggestTriggerInfo | null {
		const line = editor.getLine(cursor.line).slice(0, cursor.ch);
		const match = line.match(/#[A-Za-z0-9_-]*$/);
		if (!match) return null;

		return {
			start: { line: cursor.line, ch: cursor.ch - match[0].length },
			end: cursor,
			query: match[0],
		};
	}

	getSuggestions(context: EditorSuggestContext): string[] {
		const query = context.query.toLowerCase();
		return this.vaultIndex.tags().filter((tag) => tag.toLowerCase().startsWith(query));
	}

	renderSuggestion(tag: string, el: HTMLElement): void {
		el.setText(tag);
	}

	selectSuggestion(tag: string, _evt: MouseEvent | KeyboardEvent): void {
		const context = this.context;
		if (!context) return;
		context.editor.replaceRange(tag, context.start, context.end);
	}
}

import { ExcerptDirective, ExcerptError } from "./types";
import { isAbsolute } from "path";

export function parseExcerptDirective(source: string): ExcerptDirective {
	const lines = source.trim().split("\n");
	const fields: Record<string, string> = {};

	for (const line of lines) {
		const match = line.match(/^\s*(\w+)\s*:\s*(.+)\s*$/);
		if (match) {
			fields[match[1]!.toLowerCase()] = match[2]!.trim();
		}
	}

	const filePath = fields["file"];
	if (!filePath) {
		throw new ExcerptError("Missing required 'file' field in excerpt directive");
	}

	const directive: ExcerptDirective = {
		filePath,
		isAbsolutePath: isAbsolute(filePath),
	};

	if (fields["heading"]) {
		directive.heading = fields["heading"];
	}

	if (fields["lines"]) {
		const rangeMatch = fields["lines"].match(/^(\d+)\s*-\s*(\d+)$/);
		if (!rangeMatch) {
			throw new ExcerptError("Invalid 'lines' format. Expected 'start-end' (e.g., '10-25')");
		}
		directive.lineStart = parseInt(rangeMatch[1]!, 10);
		directive.lineEnd = parseInt(rangeMatch[2]!, 10);
		if (directive.lineStart > directive.lineEnd) {
			throw new ExcerptError("Line start must be less than or equal to line end");
		}
	}

	return directive;
}

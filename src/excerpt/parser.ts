import { ExcerptDirective, ExcerptError } from "./types";

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
	const lines_param = fields["lines"];
	if (!lines_param) {
		throw new ExcerptError("Missing required 'lines' field in excerpt directive");
	}

	const rangeMatch = lines_param.match(/^(\d+)\s*-\s*(\d+)$/);
	if (!rangeMatch) {
		throw new ExcerptError("Invalid 'lines' format. Expected 'start-end' (e.g., '10-25')");
	}

	const lineStart = parseInt(rangeMatch[1]!, 10);
	const lineEnd = parseInt(rangeMatch[2]!, 10);
	if (lineStart > lineEnd) {
		throw new ExcerptError("Line start must be less than or equal to line end");
	}
	const directive: ExcerptDirective = {
		filePath,
		lineStart,
		lineEnd
	};



	return directive;
}

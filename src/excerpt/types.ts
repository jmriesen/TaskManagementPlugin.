export interface ExcerptDirective {
	filePath: string;
	heading?: string;
	lineStart?: number;
	lineEnd?: number;
}

export interface ExcerptResult {
	content: string;
	sourcePath: string;
}

export class ExcerptError extends Error {
	constructor(message: string) {
		super(message);
		this.name = "ExcerptError";
	}
}

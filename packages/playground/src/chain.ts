// A safe, hand-written parser for the playground's method-chain mini-language.
// It never uses eval / new Function: the chain is scanned and parsed into a
// `ParsedChain`, which the evaluator dispatches through the operation registry.
//
// Grammar:
//   chain   := source ('.' method)*
//   source  := ('parse' | 'fromText' | 'fromLatex' | 'fromAst') '(' literal ')'
//   method  := IDENT '(' argList? ')'
//   argList := literal (',' literal)*
//   literal := STRING | NUMBER | BOOLEAN | array | object
//   array   := '[' (literal (',' literal)*)? ']'
//   object  := '{' (pair (',' pair)*)? '}'
//   pair    := (IDENT | STRING) ':' literal

import type {
  ChainSource,
  ChainStep,
  Literal,
  ParseOutcome,
  ParsedChain,
  SourceKind,
  Span,
} from "./types";

const SOURCES: readonly SourceKind[] = [
  "parse",
  "fromText",
  "fromLatex",
  "fromAst",
];

/** The variable name the first box's equation is stored under. */
export const BASE_VAR = "expr";

/** An internal, located parse failure. Caught at the top level. */
class ParseError {
  constructor(
    readonly message: string,
    readonly start: number,
    readonly end: number,
  ) {}
}

type TokKind = "ident" | "string" | "number" | "boolean" | "punct" | "eof";
interface Tok {
  kind: TokKind;
  value: string;
  start: number;
  end: number;
}

const isSpace = (c: string) => c === " " || c === "\t" || c === "\n" || c === "\r";
const isDigit = (c: string) => c >= "0" && c <= "9";
const isIdentStart = (c: string) =>
  (c >= "a" && c <= "z") || (c >= "A" && c <= "Z") || c === "_" || c === "$";
const isIdentPart = (c: string) => isIdentStart(c) || isDigit(c);
const PUNCT = new Set(["(", ")", ".", ",", "[", "]", "{", "}", ":"]);

/** Scan the input into a token list terminated by an `eof` token. */
function tokenize(src: string): Tok[] {
  const toks: Tok[] = [];
  let i = 0;
  const n = src.length;
  while (i < n) {
    const c = src[i];
    if (isSpace(c)) {
      i++;
      continue;
    }
    // string literal
    if (c === '"' || c === "'") {
      const start = i;
      const quote = c;
      i++;
      let value = "";
      while (i < n && src[i] !== quote) {
        if (src[i] === "\\" && i + 1 < n) {
          const esc = src[i + 1];
          value +=
            esc === "n" ? "\n" : esc === "t" ? "\t" : esc === "r" ? "\r" : esc;
          i += 2;
        } else {
          value += src[i];
          i++;
        }
      }
      if (i >= n)
        throw new ParseError("unterminated string literal", start, n);
      i++; // closing quote
      toks.push({ kind: "string", value, start, end: i });
      continue;
    }
    // number literal (a leading sign is unambiguous — the grammar has no binary minus)
    if (
      isDigit(c) ||
      (c === "." && isDigit(src[i + 1] ?? "")) ||
      ((c === "-" || c === "+") && (isDigit(src[i + 1] ?? "") || src[i + 1] === "."))
    ) {
      const start = i;
      if (c === "-" || c === "+") i++;
      while (i < n && isDigit(src[i])) i++;
      if (src[i] === ".") {
        i++;
        while (i < n && isDigit(src[i])) i++;
      }
      if (src[i] === "e" || src[i] === "E") {
        i++;
        if (src[i] === "-" || src[i] === "+") i++;
        while (i < n && isDigit(src[i])) i++;
      }
      toks.push({ kind: "number", value: src.slice(start, i), start, end: i });
      continue;
    }
    // identifier / boolean keyword
    if (isIdentStart(c)) {
      const start = i;
      i++;
      while (i < n && isIdentPart(src[i])) i++;
      const value = src.slice(start, i);
      const kind: TokKind =
        value === "true" || value === "false" ? "boolean" : "ident";
      toks.push({ kind, value, start, end: i });
      continue;
    }
    // punctuation
    if (PUNCT.has(c)) {
      toks.push({ kind: "punct", value: c, start: i, end: i + 1 });
      i++;
      continue;
    }
    throw new ParseError(`unexpected character '${c}'`, i, i + 1);
  }
  toks.push({ kind: "eof", value: "", start: n, end: n });
  return toks;
}

/** Recursive-descent parser over the token list. */
class Parser {
  private pos = 0;
  constructor(
    private readonly toks: Tok[],
    private readonly length: number,
  ) {}

  private peek(): Tok {
    return this.toks[this.pos];
  }
  private next(): Tok {
    return this.toks[this.pos++];
  }
  private isPunct(v: string): boolean {
    const t = this.peek();
    return t.kind === "punct" && t.value === v;
  }
  private expectPunct(v: string, what: string): Tok {
    const t = this.peek();
    if (t.kind === "punct" && t.value === v) return this.next();
    throw new ParseError(`expected '${v}' ${what}`, t.start, t.end);
  }

  parseChain(): ParsedChain {
    const head = this.peek();
    if (head.kind !== "ident") {
      throw new ParseError(
        `a chain must start with the variable "${BASE_VAR}" or a source: parse(…), fromText(…), fromLatex(…) or fromAst(…)`,
        head.start,
        head.end,
      );
    }

    let source: ChainSource;
    if (SOURCES.includes(head.value as SourceKind)) {
      // factory call, e.g. parse("x^2 - 1")
      this.next();
      this.expectPunct("(", `after ${head.value}`);
      const arg = this.parseLiteral();
      const close = this.expectPunct(")", `to close ${head.value}(…)`);
      source = {
        kind: head.value as SourceKind,
        arg,
        span: { start: head.start, end: close.end } satisfies Span,
      };
    } else {
      // bare variable reference (validated against BASE_VAR by the evaluator)
      this.next();
      source = {
        kind: "var",
        name: head.value,
        span: { start: head.start, end: head.end } satisfies Span,
      };
    }

    const steps: ChainStep[] = [];
    while (this.isPunct(".")) {
      this.next(); // '.'
      const name = this.peek();
      if (name.kind !== "ident")
        throw new ParseError(
          "expected a method name after '.'",
          name.start,
          name.end,
        );
      this.next();
      this.expectPunct("(", `after .${name.value}`);
      const args = this.parseArgList();
      const stepClose = this.expectPunct(")", `to close .${name.value}(…)`);
      steps.push({
        method: name.value,
        args,
        nameSpan: { start: name.start, end: name.end },
        span: { start: name.start, end: stepClose.end },
      });
    }

    const end = this.peek();
    if (end.kind !== "eof")
      throw new ParseError("unexpected trailing input", end.start, this.length);

    return { source, steps };
  }

  private parseArgList(): Literal[] {
    if (this.isPunct(")")) return [];
    const args: Literal[] = [this.parseLiteral()];
    while (this.isPunct(",")) {
      this.next();
      args.push(this.parseLiteral());
    }
    return args;
  }

  private parseLiteral(): Literal {
    const t = this.peek();
    switch (t.kind) {
      case "string":
        this.next();
        return { kind: "string", value: t.value, span: { start: t.start, end: t.end } };
      case "number": {
        this.next();
        const value = Number(t.value);
        if (!Number.isFinite(value))
          throw new ParseError(`invalid number '${t.value}'`, t.start, t.end);
        return { kind: "number", value, span: { start: t.start, end: t.end } };
      }
      case "boolean":
        this.next();
        return {
          kind: "boolean",
          value: t.value === "true",
          span: { start: t.start, end: t.end },
        };
      case "punct":
        if (t.value === "[") return this.parseArray();
        if (t.value === "{") return this.parseObject();
        break;
    }
    throw new ParseError(
      "expected a literal (string, number, boolean, array, or object)",
      t.start,
      t.end,
    );
  }

  private parseArray(): Literal {
    const open = this.next(); // '['
    const items: Literal[] = [];
    if (!this.isPunct("]")) {
      items.push(this.parseLiteral());
      while (this.isPunct(",")) {
        this.next();
        items.push(this.parseLiteral());
      }
    }
    const close = this.expectPunct("]", "to close the array");
    return { kind: "array", items, span: { start: open.start, end: close.end } };
  }

  private parseObject(): Literal {
    const open = this.next(); // '{'
    const entries: { key: string; value: Literal }[] = [];
    if (!this.isPunct("}")) {
      entries.push(this.parsePair());
      while (this.isPunct(",")) {
        this.next();
        entries.push(this.parsePair());
      }
    }
    const close = this.expectPunct("}", "to close the object");
    return {
      kind: "object",
      entries,
      span: { start: open.start, end: close.end },
    };
  }

  private parsePair(): { key: string; value: Literal } {
    const k = this.peek();
    if (k.kind !== "ident" && k.kind !== "string")
      throw new ParseError("expected an object key", k.start, k.end);
    this.next();
    this.expectPunct(":", `after key '${k.value}'`);
    return { key: k.value, value: this.parseLiteral() };
  }
}

/** Parse a chain string into a `ParsedChain` or a located error. */
export function parseChain(src: string): ParseOutcome {
  try {
    const toks = tokenize(src);
    const chain = new Parser(toks, src.length).parseChain();
    return { ok: true, chain };
  } catch (e) {
    if (e instanceof ParseError)
      return { ok: false, error: { message: e.message, start: e.start, end: e.end } };
    throw e;
  }
}

/** Convert a parsed `Literal` to a plain JS value (for `fromAst` / arg readers). */
export function literalToValue(lit: Literal): unknown {
  switch (lit.kind) {
    case "string":
    case "number":
    case "boolean":
      return lit.value;
    case "array":
      return lit.items.map(literalToValue);
    case "object": {
      const o: Record<string, unknown> = {};
      for (const { key, value } of lit.entries) o[key] = literalToValue(value);
      return o;
    }
  }
}

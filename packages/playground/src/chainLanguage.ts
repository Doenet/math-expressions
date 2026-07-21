// Syntax highlighting for the method-chain mini-language. A tiny
// `StreamLanguage` tokenizer classifies source factories, method names (any
// identifier right after a `.`), string/number literals, and booleans; a
// `HighlightStyle` colours them to match the app's AST-tree palette.

import {
  HighlightStyle,
  StreamLanguage,
  type StringStream,
  syntaxHighlighting,
} from "@codemirror/language";
import { tags as t } from "@lezer/highlight";
import type { Extension } from "@codemirror/state";
import { BASE_VAR } from "./chain";

const SOURCES = new Set(["parse", "fromText", "fromLatex", "fromAst"]);

export interface ChainTokState {
  afterDot: boolean;
}

/**
 * Classify the next token. Exported (rather than inlined in the StreamLanguage)
 * so the classification can be unit-tested with a bare StringStream.
 */
export function chainToken(
  stream: StringStream,
  state: ChainTokState,
): string | null {
  if (stream.eatSpace()) return null;
  const ch = stream.peek();

  // string literal (the math expression lives inside these quotes)
  if (ch === '"' || ch === "'") {
    stream.next();
    let escaped = false;
    let c: string | void;
    while ((c = stream.next()) != null) {
      if (c === ch && !escaped) break;
      escaped = c === "\\" && !escaped;
    }
    state.afterDot = false;
    return "str";
  }

  // number literal
  if (ch && ch >= "0" && ch <= "9") {
    stream.match(/^[0-9]*\.?[0-9]+([eE][+-]?[0-9]+)?/);
    state.afterDot = false;
    return "num";
  }

  // identifier: source factory, boolean, method name, or bare variable
  if (ch && /[A-Za-z_$]/.test(ch)) {
    const m = stream.match(/^[A-Za-z_$][\w$]*/);
    const word = Array.isArray(m) ? m[0] : "";
    if (state.afterDot) {
      state.afterDot = false;
      return "method";
    }
    if (word === BASE_VAR) return "base";
    if (SOURCES.has(word)) return "source";
    if (word === "true" || word === "false") return "bool";
    return "variable";
  }

  // dot: either the chain operator, or a leading decimal point
  if (ch === ".") {
    stream.next();
    if (stream.match(/^[0-9]+([eE][+-]?[0-9]+)?/)) {
      state.afterDot = false;
      return "num";
    }
    state.afterDot = true;
    return "punctuation";
  }

  // any other punctuation
  stream.next();
  state.afterDot = false;
  return "punctuation";
}

const chainStream = StreamLanguage.define<ChainTokState>({
  startState: () => ({ afterDot: false }),
  copyState: (s) => ({ afterDot: s.afterDot }),
  token: chainToken,
  tokenTable: {
    source: t.keyword,
    method: t.propertyName,
    base: t.className,
    str: t.string,
    num: t.number,
    bool: t.bool,
    variable: t.variableName,
    punctuation: t.punctuation,
  },
});

// Colours mirror the AST-tree palette in styles.css for a cohesive look.
const chainHighlight = HighlightStyle.define([
  { tag: t.keyword, color: "#be185d", fontWeight: "600" }, // parse / fromText / …
  { tag: t.className, color: "#6d28d9", fontWeight: "600" }, // expr (the stored equation)
  { tag: t.propertyName, color: "#047857" }, // .method
  { tag: t.string, color: "#b7791f" }, // "math expression"
  { tag: t.number, color: "#2563eb" },
  { tag: t.bool, color: "#c2410c" },
  { tag: t.variableName, color: "#1c2230" },
  { tag: t.punctuation, color: "#667085" },
]);

/** The chain language + its highlight style, ready to drop into an editor. */
export const chainLanguage: Extension = [
  chainStream,
  syntaxHighlighting(chainHighlight),
];

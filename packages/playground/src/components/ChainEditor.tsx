// A CodeMirror 6 editor for the method-chain mini-language. It provides:
//   - registry-driven autocomplete, opened on typing `.` (or Ctrl-Space),
//   - a linter that underlines the current parse error (via parseChain),
//   - auto-closing of quotes/brackets and standard editing keymaps.
// It keeps the original `ChainEditor` interface: a controlled `value`/`onChange`
// plus an imperative `insertAtCursor` used by the operation palette.

import { forwardRef, useEffect, useImperativeHandle, useRef } from "react";
import { EditorState } from "@codemirror/state";
import { EditorView, drawSelection, keymap, placeholder } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import {
  autocompletion,
  closeBrackets,
  closeBracketsKeymap,
  completionKeymap,
  startCompletion,
  type CompletionContext,
  type CompletionResult,
} from "@codemirror/autocomplete";
import { type Diagnostic, linter, lintKeymap } from "@codemirror/lint";
import { BASE_VAR, parseChain } from "../chain";
import { chainLanguage } from "../chainLanguage";
import type { OpEntry } from "../types";

export interface ChainEditorHandle {
  /** Insert text at the caret (or over the selection) and refocus. */
  insertAtCursor(text: string): void;
}

interface Props {
  value: string;
  onChange: (v: string) => void;
  /** Operations offered in `.`-autocomplete (curated registry + dynamic ops). */
  ops: OpEntry[];
}

/** Insert `text` over the completion range, placing the caret at `caret`. */
function replaceWith(view: EditorView, from: number, to: number, text: string, caret: number) {
  view.dispatch({
    changes: { from, to, insert: text },
    selection: { anchor: caret },
  });
}

/** Sources offered at the start of a chain: the stored equation, or a factory. */
const SOURCE_COMPLETIONS = [
  { label: BASE_VAR, detail: "stored equation", type: "variable", snippet: BASE_VAR },
  ...(["parse", "fromText", "fromLatex", "fromAst"] as const).map((f) => ({
    label: f,
    detail: "source",
    type: "keyword",
    snippet: `${f}("")`,
  })),
];

/** Whether the cursor sits inside an unterminated string literal. */
function insideString(text: string): boolean {
  let quote: string | null = null;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (quote) {
      if (c === "\\") i++; // skip the escaped char
      else if (c === quote) quote = null;
    } else if (c === '"' || c === "'") {
      quote = c;
    }
  }
  return quote !== null;
}

/**
 * Autocomplete: method names after a `.`, or a source (`expr` / `parse(…)`) at
 * the very start of the chain. `opsRef` is read at completion time so newly
 * loaded dynamic ops appear without rebuilding the editor.
 */
function makeChainCompletions(opsRef: { current: OpEntry[] }) {
  return function chainCompletions(
    context: CompletionContext,
  ): CompletionResult | null {
  // Never complete inside a string literal (e.g. a `.` within parse("a.b")).
  if (insideString(context.state.sliceDoc(0, context.pos))) return null;

  // Method completions right after a dot.
  const dot = context.matchBefore(/\.[\w$]*$/);
  if (dot) {
    return {
      from: dot.from + 1, // keep the dot, replace the partial after it
      options: opsRef.current.map((e) => ({
        label: e.id,
        detail: e.category,
        type: e.js && e.rust ? "function" : "method",
        apply: (view: EditorView, _c: unknown, aFrom: number, aTo: number) => {
          const openParen = e.insertText.indexOf("(");
          const caret =
            e.args.length > 0 && openParen >= 0
              ? aFrom + openParen + 1 // inside the parens when there are args
              : aFrom + e.insertText.length;
          replaceWith(view, aFrom, aTo, e.insertText, caret);
        },
      })),
    };
  }

  // Source completions when the whole prefix is just an optional leading word.
  const atStart = context.matchBefore(/^\s*[\w$]*$/);
  if (!atStart) return null;
  const word = context.matchBefore(/[\w$]*$/);
  if (!word || (!context.explicit && word.text === "")) return null;
  return {
    from: word.from,
    options: SOURCE_COMPLETIONS.map((s) => ({
      label: s.label,
      detail: s.detail,
      type: s.type,
      apply: (view: EditorView, _c: unknown, aFrom: number, aTo: number) => {
        // Drop the caret inside the quotes for a factory call, else after `expr`.
        const caret = s.snippet.endsWith('("")')
          ? aFrom + s.snippet.length - 2
          : aFrom + s.snippet.length;
        replaceWith(view, aFrom, aTo, s.snippet, caret);
      },
    })),
  };
  };
}

/** Surface the current parse error as an underlined diagnostic. */
const chainLinter = linter((view: EditorView): Diagnostic[] => {
  const doc = view.state.doc.toString();
  if (doc.trim() === "") return [];
  const r = parseChain(doc);
  if (r.ok) return [];
  const from = Math.max(0, Math.min(r.error.start, doc.length));
  let to = Math.max(from, Math.min(r.error.end, doc.length));
  if (to === from) to = Math.min(doc.length, from + 1);
  return [{ from, to, severity: "error", message: r.error.message }];
});

const editorTheme = EditorView.theme({
  "&": {
    fontSize: "16px",
    background: "var(--card2)",
    border: "1px solid var(--border)",
    borderRadius: "8px",
    color: "var(--fg)",
  },
  "&.cm-focused": { outline: "2px solid var(--accent)", outlineOffset: "-1px" },
  ".cm-content": {
    fontFamily: "var(--mono)",
    padding: "9px 12px",
    caretColor: "var(--accent)",
    minHeight: "60px",
  },
  ".cm-line": { padding: "0" },
  "&.cm-editor .cm-scroller": { fontFamily: "var(--mono)", lineHeight: "1.6" },
});

const ChainEditor = forwardRef<ChainEditorHandle, Props>(function ChainEditor(
  { value, onChange, ops },
  ref,
) {
  const host = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;
  // Read by the completion source at request time, so dynamically loaded ops
  // become available without recreating the editor.
  const opsRef = useRef(ops);
  opsRef.current = ops;

  // Create the editor once.
  useEffect(() => {
    if (!host.current) return;
    const view = new EditorView({
      parent: host.current,
      state: EditorState.create({
        doc: value,
        extensions: [
          history(),
          drawSelection(),
          EditorView.lineWrapping,
          chainLanguage,
          closeBrackets(),
          autocompletion({
            override: [makeChainCompletions(opsRef)],
            activateOnTyping: true,
          }),
          chainLinter,
          placeholder('parse("x^2 - 1").reduce_rational().toLatex()'),
          keymap.of([
            ...closeBracketsKeymap,
            ...defaultKeymap,
            ...historyKeymap,
            ...completionKeymap,
            ...lintKeymap,
          ]),
          editorTheme,
          EditorView.updateListener.of((u) => {
            if (!u.docChanged) return;
            onChangeRef.current(u.state.doc.toString());
            // Open the completion popup right after a `.` is typed.
            let typedDot = false;
            u.changes.iterChanges((_fa, _ta, _fb, _tb, inserted) => {
              if (inserted.toString().endsWith(".")) typedDot = true;
            });
            if (typedDot) startCompletion(u.view);
          }),
        ],
      }),
    });
    viewRef.current = view;
    return () => {
      view.destroy();
      viewRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Sync external value changes (examples, palette inserts) into the editor.
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    const cur = view.state.doc.toString();
    if (value !== cur) {
      view.dispatch({ changes: { from: 0, to: cur.length, insert: value } });
    }
  }, [value]);

  useImperativeHandle(ref, () => ({
    insertAtCursor(text: string) {
      const view = viewRef.current;
      if (!view) return;
      const { from, to } = view.state.selection.main;
      view.dispatch({
        changes: { from, to, insert: text },
        selection: { anchor: from + text.length },
      });
      view.focus();
    },
  }));

  return <div className="editor-wrap" ref={host} />;
});

export default ChainEditor;

import { useDeferredValue, useEffect, useMemo, useRef, useState } from "react";
import {
  loadEngines,
  makeJsAdapter,
  makeRustAdapter,
  type LoadedEngines,
} from "./engines";
import { BASE_VAR, parseChain } from "./chain";
import { evaluateChain } from "./evaluate";
import { CATEGORIES, REGISTRY } from "./registry";
import { formatComplex, formatFloat, safe } from "./util";
import Katex from "./components/Katex";
import Tree from "./components/Tree";
import ChainEditor, { type ChainEditorHandle } from "./components/ChainEditor";
import type {
  Notation,
  Displayable,
  OpCategory,
  SafeResult,
  StepResult,
  Syntax,
} from "./types";

/* ------------------------------ examples ------------------------------ */

// Examples operate on the stored `expr`; two show calling a source directly,
// which ignores the equation box.
const EXAMPLES: { label: string; chain: string }[] = [
  { label: "simplify", chain: `${BASE_VAR}.simplify()` },
  { label: "derivative", chain: `${BASE_VAR}.derivative("x").simplify()` },
  { label: "expand→latex", chain: `${BASE_VAR}.expand().toLatex()` },
  { label: "evaluate", chain: `${BASE_VAR}.evaluate({x: "2 + 3i"})` },
  { label: "variables", chain: `${BASE_VAR}.variables()` },
  { label: "integrate", chain: `${BASE_VAR}.integrate("x")` },
  {
    label: "literal parse",
    chain: 'parse("x^2 - 1").divide("x - 1").reduce_rational()',
  },
  {
    label: "from LaTeX",
    chain: 'fromLatex("\\\\frac{x}{y} + \\\\sqrt{x}").variables()',
  },
];

// Curated equations (+ the chain that reveals the difference) where the Rust
// engine does something the canonical JS library cannot — symbolic integration,
// factoring, certified zero-testing, or deeper simplification.
const SHOWCASE: { label: string; expr: string; chain: string }[] = [
  {
    label: "Pythagorean identity  →  2",
    expr: "sin^2(x) + cos^2(x) + 1",
    chain: `${BASE_VAR}.simplify()`,
  },
  {
    label: "e^(ln x)  collapses to  x",
    expr: "e^(log(x))",
    chain: `${BASE_VAR}.simplify()`,
  },
  {
    label: "∫ 1/(1+x²) dx  →  arctan  (Rust only)",
    expr: "1/(1 + x^2)",
    chain: `${BASE_VAR}.integrate("x")`,
  },
  {
    label: "∫ 1/x dx  →  ln|x|  (Rust only)",
    expr: "1/x",
    chain: `${BASE_VAR}.integrate("x")`,
  },
  {
    label: "Factor  x⁴ − 1  over ℚ  (Rust only)",
    expr: "x^4 - 1",
    chain: `${BASE_VAR}.factor()`,
  },
  {
    label: "Certified  (x+1)² − x² − 2x − 1 = 0  (Rust only)",
    expr: "(x + 1)^2 - x^2 - 2x - 1",
    chain: `${BASE_VAR}.is_zero()`,
  },
  {
    label: "√2 to 30 digits  (Rust exact; JS ~16)",
    expr: "sqrt(2)",
    chain: `${BASE_VAR}.evaluate_to_precision(30)`,
  },
];

// The single Examples menu shown below the editor: plain chains (applied to the
// current equation) plus the Rust-showcase entries (which also set the
// equation + its syntax).
type MenuItem = { label: string; chain: string; expr?: string; syntax?: Syntax };
const EXAMPLE_MENU: { group: string; items: MenuItem[] }[] = [
  { group: "Chains (on the current equation)", items: EXAMPLES },
  {
    group: "Rust showcase (sets the equation too)",
    items: SHOWCASE.map((s) => ({
      label: s.label,
      chain: s.chain,
      expr: s.expr,
      syntax: "text" as Syntax,
    })),
  },
];
const MENU_FLAT: MenuItem[] = EXAMPLE_MENU.flatMap((g) => g.items);

/* ----------------------------- result views ---------------------------- */

/** Render one engine's displayable result for a step. */
function DisplayView({ d }: { d: Displayable }) {
  switch (d.kind) {
    case "expression":
      return (
        <div className="disp">
          <div className="rendered">
            <Katex tex={d.latex} display />
          </div>
          <div className="kv">
            <span className="k">text</span>
            <code>{d.text}</code>
          </div>
          <details>
            <summary>tree</summary>
            <div className="tree-box">
              <Tree value={d.tree} />
            </div>
          </details>
        </div>
      );
    case "string":
      return d.render === "latex" ? (
        <div className="rendered">
          <Katex tex={d.value} display />
        </div>
      ) : (
        <code className="block">{d.value}</code>
      );
    case "stringList":
      return d.value.length ? (
        <div className="chips">
          {d.value.map((v) => (
            <span key={v} className="chip">
              {v}
            </span>
          ))}
        </div>
      ) : (
        <span className="muted">none</span>
      );
    case "boolean":
      return <span className={"bool " + d.value}>{String(d.value)}</span>;
    case "number":
      return d.value === null ? (
        <span className="muted">undefined</span>
      ) : (
        <span className="num">{formatFloat(d.value)}</span>
      );
    case "complex":
      return d.value === null ? (
        <span className="muted">undefined</span>
      ) : (
        <span className="num">{formatComplex(d.value)}</span>
      );
    case "tree":
      return (
        <div className="tree-box">
          <Tree value={d.value} />
        </div>
      );
    case "none":
      return <span className="muted">no result</span>;
    case "unsupported":
      return (
        <span className="muted na" title={d.reason}>
          unsupported
        </span>
      );
    case "ended":
      return <span className="muted">—</span>;
  }
}

/** One engine's cell: a guarded displayable, or an error. */
function Cell({
  res,
  call,
}: {
  res: SafeResult<Displayable>;
  call?: string;
}) {
  return (
    <div className="cell">
      {res.ok ? (
        <DisplayView d={res.value} />
      ) : (
        <span className="err" title={res.error}>
          error: {res.error}
        </span>
      )}
      {call && (
        <div className="call" title="underlying call on this engine">
          {call}
        </div>
      )}
    </div>
  );
}

function AgreeBadge({ agree }: { agree?: boolean }) {
  if (agree === undefined) return null;
  return agree ? (
    <span className="badge ok">agree ✓</span>
  ) : (
    <span className="badge diff">differ ✗</span>
  );
}

/** A compact rendering used in the collapsed (agreeing) view: just the math. */
function AbbrevView({ d }: { d: Displayable }) {
  switch (d.kind) {
    case "expression":
      return <Katex tex={d.latex} display />;
    case "string":
      return d.render === "latex" ? (
        <Katex tex={d.value} display />
      ) : (
        <code>{d.value}</code>
      );
    default:
      // terminals (list / bool / number / complex / tree / none) are already compact
      return <DisplayView d={d} />;
  }
}

function StepLabel({ step, index }: { step: StepResult; index: number }) {
  return (
    <h3>
      <span className="step-num">{index === 0 ? "source" : `#${index}`}</span>
      <code className="step-label">
        {index === 0 ? step.label : `.${step.label}()`}
      </code>
    </h3>
  );
}

function StepColumns({ step }: { step: StepResult }) {
  return (
    <div className="cols">
      <div className="col">
        <h4>JavaScript</h4>
        <Cell res={step.jsResult} call={step.call.js} />
      </div>
      <div className="col">
        <h4>Rust (WASM)</h4>
        <Cell res={step.rustResult} call={step.call.rust} />
      </div>
    </div>
  );
}

function StepCard({ step, index }: { step: StepResult; index: number }) {
  // When both engines agree, collapse to an abbreviated math-only view that
  // expands to the full side-by-side comparison on demand.
  if (step.agree === true && step.jsResult.ok) {
    return (
      <section className="card step agree">
        <details className="agree-collapse">
          <summary>
            <StepLabel step={step} index={index} />
            <span className="abbrev">
              <AbbrevView d={step.jsResult.value} />
            </span>
            <AgreeBadge agree />
            <span className="expand-hint">compare</span>
          </summary>
          <StepColumns step={step} />
        </details>
      </section>
    );
  }
  return (
    <section className="card step">
      <div className="section-head">
        <StepLabel step={step} index={index} />
        <AgreeBadge agree={step.agree} />
      </div>
      <StepColumns step={step} />
    </section>
  );
}

/* -------------------------------- app --------------------------------- */

export default function App() {
  const [loaded, setLoaded] = useState<LoadedEngines | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [baseText, setBaseText] = useState("sin^2(x) + cos^2(x) + 1");
  const [baseSyntax, setBaseSyntax] = useState<Syntax>("text");
  const [notation, setNotation] = useState<Notation>("period");
  const [chain, setChain] = useState(EXAMPLES[0].chain);
  const editorRef = useRef<ChainEditorHandle>(null);

  useEffect(() => {
    loadEngines().then(setLoaded, (e: unknown) =>
      setLoadError(e instanceof Error ? e.message : String(e)),
    );
  }, []);

  // Notation-specific engine adapters, rebuilt when the separator convention
  // changes (Rust parses natively; the JS adapter transliterates).
  const engines = useMemo(
    () =>
      loaded
        ? {
            js: makeJsAdapter(notation),
            rust: makeRustAdapter(loaded.rustModule, notation),
          }
        : null,
    [loaded, notation],
  );

  // Live preview of the equation box: parse with the JS engine and render it.
  const basePreview = useMemo(() => {
    if (!engines || baseText.trim() === "") return null;
    return safe(() => {
      const h =
        baseSyntax === "latex"
          ? engines.js.fromLatex(baseText)
          : engines.js.fromText(baseText);
      return { latex: engines.js.toLatex(h), text: engines.js.toText(h) };
    });
  }, [engines, baseText, baseSyntax]);

  // Parse the *live* chain text for the editor's error underline.
  const liveParse = useMemo(() => parseChain(chain), [chain]);
  const editorError = liveParse.ok ? null : liveParse.error;

  // Evaluate at deferred priority so typing stays responsive.
  const deferredChain = useDeferredValue(chain);
  const deferredBase = useDeferredValue(baseText);
  const deferredSyntax = useDeferredValue(baseSyntax);
  const isStale =
    deferredChain !== chain ||
    deferredBase !== baseText ||
    deferredSyntax !== baseSyntax;
  const evaluation = useMemo(() => {
    if (!engines) return null;
    const parsed = parseChain(deferredChain);
    if (!parsed.ok) return { error: parsed.error, steps: null as StepResult[] | null };
    return {
      error: null,
      steps: evaluateChain(engines.js, engines.rust, parsed.chain, {
        text: deferredBase,
        syntax: deferredSyntax,
      }),
    };
  }, [engines, deferredChain, deferredBase, deferredSyntax]);

  const byCategory = useMemo(() => {
    const m = new Map<OpCategory, typeof REGISTRY>();
    for (const c of CATEGORIES) m.set(c, []);
    for (const e of REGISTRY) m.get(e.category)!.push(e);
    return m;
  }, []);

  if (loadError)
    return (
      <div className="app">
        <h1>Failed to load engines</h1>
        <pre className="err">{loadError}</pre>
      </div>
    );

  return (
    <div className="app">
      <header>
        <h1>math-expressions playground</h1>
        <p className="sub">
          Enter an equation, then chain operations on it — run against the
          canonical <b>JavaScript</b> library and the <b>Rust (WASM)</b> port,
          side by side.
        </p>
      </header>

      {/* ---- box 1: the equation ---- */}
      <section className="card">
        <div className="section-head">
          <h2>
            Equation <span className="muted">— stored as</span>{" "}
            <code>{BASE_VAR}</code>
          </h2>
          <div className="eq-controls">
            Decimal Format:{" "}
            <select
              className="showcase"
              value={notation}
              onChange={(e) => setNotation(e.target.value as Notation)}
              title="Decimal / argument separators for math strings"
            >
              <option value="period">1.5, x&nbsp;&nbsp;(. ,)</option>
              <option value="comma">1,5; x&nbsp;&nbsp;(, ;)</option>
            </select>
            <div className="radios">
              <label>
                <input
                  type="radio"
                  checked={baseSyntax === "text"}
                  onChange={() => setBaseSyntax("text")}
                />{" "}
                text
              </label>
              <label>
                <input
                  type="radio"
                  checked={baseSyntax === "latex"}
                  onChange={() => setBaseSyntax("latex")}
                />{" "}
                LaTeX
              </label>
            </div>
          </div>
        </div>
        <div className="io-split">
          <input
            className="expr-input"
            value={baseText}
            spellCheck={false}
            onChange={(e) => setBaseText(e.target.value)}
            placeholder={
              baseSyntax === "latex" ? "\\frac{x}{y} + \\sqrt{x}" : "x^2 - 1"
            }
          />
          <div className="base-preview">
            {basePreview == null ? (
              <span className="muted">—</span>
            ) : basePreview.ok ? (
              <>
                <Katex tex={basePreview.value.latex} display />
                <code className="muted">{basePreview.value.text}</code>
              </>
            ) : (
              <span className="err" title={basePreview.error}>
                parse error: {basePreview.error}
              </span>
            )}
          </div>
        </div>
        {notation === "comma" && (
          <p className="muted eq-note">
            Comma notation (<code>1,5</code>, <code>f(x; y)</code>) applies inside
            math strings. Rust parses it natively; the JS engine has no
            comma-decimal parser, so it is transliterated.
          </p>
        )}
      </section>

      {/* ---- box 2: the chain, paired with the operations palette ---- */}
      <div className="chain-split">
      <section className="card chain-card">
        <div className="section-head">
          <h2>Evaluate</h2>
          <span className="muted">
            Start from <code>{BASE_VAR}</code> and evalute the following math.
          </span>
        </div>
        <ChainEditor ref={editorRef} value={chain} onChange={setChain} />
        {editorError && (
          <p className="editor-err">
            <span className="err">✗ {editorError.message}</span>{" "}
            <span className="muted">(at position {editorError.start})</span>
          </p>
        )}
        <div className="examples-row">
          <select
            className="showcase"
            value=""
            onChange={(e) => {
              const it = MENU_FLAT[Number(e.target.value)];
              if (!it) return;
              if (it.expr !== undefined) {
                setBaseText(it.expr);
                setBaseSyntax(it.syntax ?? "text");
              }
              setChain(it.chain);
            }}
            title="Load an example"
          >
            <option value="" disabled>
              Examples…
            </option>
            {(() => {
              let idx = -1;
              return EXAMPLE_MENU.map((g) => (
                <optgroup key={g.group} label={g.group}>
                  {g.items.map((it) => {
                    idx += 1;
                    return (
                      <option key={idx} value={idx}>
                        {it.label}
                      </option>
                    );
                  })}
                </optgroup>
              ));
            })()}
          </select>
        </div>
      </section>

      {/* ---- palette ---- */}
      <section className="card palette">
        <div className="section-head">
          <h2>Operations</h2>
          <span className="muted">
            click to append at the cursor · type <code>.</code> in the editor for
            autocomplete
          </span>
        </div>
        {CATEGORIES.map((cat) => (
          <div key={cat} className="palette-row">
            <span className="palette-cat">{cat}</span>
            <div className="palette-ops">
              {byCategory.get(cat)!.map((e) => {
                const only =
                  e.js && e.rust ? "" : e.js ? " js-only" : " rust-only";
                return (
                  <button
                    key={e.id}
                    className={"op-btn" + only}
                    title={
                      (e.js ? "" : "JS: " + (e.unsupportedReason?.js ?? "unsupported")) +
                      (e.rust ? "" : "Rust: " + (e.unsupportedReason?.rust ?? "unsupported"))
                    }
                    onClick={() => editorRef.current?.insertAtCursor(`.${e.insertText}`)}
                  >
                    {e.display}
                    {only && <span className="only-tag">{only.trim().replace("-only", "")}</span>}
                  </button>
                );
              })}
            </div>
          </div>
        ))}
      </section>
      </div>

      {/* ---- results ---- */}
      {!engines ? (
        <section className="card">
          <p>Loading Rust WASM…</p>
        </section>
      ) : evaluation?.error ? (
        <section className="card">
          <p className="muted">
            Fix the chain to see results — {evaluation.error.message}.
          </p>
        </section>
      ) : (
        <div className={isStale ? "stale results-list" : "results-list"}>
          {evaluation?.steps?.map((s, i) => (
            <StepCard key={i} step={s} index={i} />
          ))}
        </div>
      )}

      <footer>
        <p className="muted">
          JS: <code>math-expressions</code> (npm) · Rust:{" "}
          <code>math-expressions-rs</code> → wasm (rebuilt via wireit when the
          Rust sources change).
        </p>
      </footer>
    </div>
  );
}

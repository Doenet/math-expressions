import { useDeferredValue, useEffect, useMemo, useState } from "react";
import { loadEngines } from "./engines";
import Katex from "./components/Katex";
import Tree from "./components/Tree";
import type {
  Analysis,
  Complex,
  Engine,
  EngineParams,
  Engines,
  SafeResult,
  Syntax,
} from "./types";

/* ----------------------------- helpers ----------------------------- */

/** Run `fn`, capturing exceptions as a tagged result. */
function safe<T>(fn: () => T): SafeResult<T> {
  try {
    return { ok: true, value: fn() };
  } catch (e) {
    return { ok: false, error: e instanceof Error ? e.message : String(e) };
  }
}

/** The value of a successful result, else undefined. */
function okValue<T>(r: SafeResult<T> | undefined): T | undefined {
  return r?.ok ? r.value : undefined;
}

function formatFloat(v: number): string {
  if (Number.isInteger(v)) return String(v);
  return String(Number(v.toPrecision(12)));
}

/** Format a `{ re, im }` value, collapsing negligible parts (a + b i / b i / a). */
function formatComplex({ re, im }: Complex): string {
  const tol = 1e-9 * Math.max(1, Math.abs(re), Math.abs(im));
  const imZero = Math.abs(im) <= tol;
  const reZero = Math.abs(re) <= tol;
  if (imZero) return formatFloat(re);
  const mag = formatFloat(Math.abs(im));
  const imPart = mag === "1" ? "i" : `${mag} i`;
  if (reZero) return (im < 0 ? "−" : "") + imPart;
  return `${formatFloat(re)} ${im < 0 ? "−" : "+"} ${imPart}`;
}

function deepEqual(a: unknown, b: unknown): boolean {
  if (Array.isArray(a) && Array.isArray(b)) {
    return a.length === b.length && a.every((x, i) => deepEqual(x, b[i]));
  }
  return a === b;
}

const EXAMPLES: { label: string; input: string; syntax: Syntax }[] = [
  { label: "sin²+cos²", input: "sin^2(x) + cos^2(x)", syntax: "text" },
  { label: "cubic", input: "x^3 + 2x - 1", syntax: "text" },
  { label: "e^(xy)", input: "e^(x y)", syntax: "text" },
  { label: "rational", input: "(x^2 - 1)/(x - 1)", syntax: "text" },
  { label: "norm", input: "sqrt(x^2 + y^2)", syntax: "text" },
  { label: "LaTeX frac", input: "\\frac{x}{y} + \\sqrt{x}", syntax: "latex" },
];

/** Analyse one expression with one engine; every step is individually guarded. */
function analyze<H>(engine: Engine<H>, params: EngineParams): Analysis {
  const { input, syntax, diffVar, bindings, assumptions, simplifyDeriv } =
    params;
  const parsed = safe(() => engine.parse(input, syntax));
  if (!parsed.ok) return { parseError: parsed.error };
  const h = parsed.value;

  const der = safe(() => engine.derivative(h, diffVar));
  const simp = safe(() => engine.simplifyWith(h, assumptions));
  // The engines' derivative() already applies their own (assumption-free)
  // simplification. When the toggle is on, additionally reduce it with the
  // playground's simplifier under the current assumptions. Value is unchanged,
  // so numeric evaluation still uses the raw derivative.
  const derShown =
    simplifyDeriv && der.ok
      ? safe(() => engine.simplifyWith(der.value, assumptions))
      : der;

  // Extract every primitive result (strings / trees / numbers) up front, while
  // the handles are still live. When a step failed, pass its {ok:false, error}
  // through so the UI can show the message instead of a silent "—".
  const result: Analysis = {
    tree: safe(() => engine.tree(h)),
    text: safe(() => engine.toText(h)),
    latex: safe(() => engine.toLatex(h)),
    variables: safe(() => engine.variables(h)),
    evalF: safe(() => engine.evaluate(h, bindings)),
    simpTree: simp.ok ? safe(() => engine.tree(simp.value)) : simp,
    simpText: simp.ok ? safe(() => engine.toText(simp.value)) : simp,
    simpLatex: simp.ok ? safe(() => engine.toLatex(simp.value)) : simp,
    derText: derShown.ok ? safe(() => engine.toText(derShown.value)) : derShown,
    derLatex: derShown.ok
      ? safe(() => engine.toLatex(derShown.value))
      : derShown,
    evalDer: der.ok ? safe(() => engine.evaluate(der.value, bindings)) : der,
  };

  // ...then free the wasm handles deterministically instead of leaking them to
  // FinalizationRegistry GC. `derShown` aliases `der` when the toggle is off,
  // so only free it when it's a distinct result.
  engine.free(h);
  if (der.ok) engine.free(der.value);
  if (simp.ok) engine.free(simp.value);
  if (derShown.ok && derShown !== der) engine.free(derShown.value);

  return result;
}

/* --------------------------- small views --------------------------- */

/** Render an evaluation result (`safe` wrapping `Complex | null`). */
function EvalValue({ res }: { res?: SafeResult<Complex | null> }) {
  if (!res) return <span className="muted">—</span>;
  if (!res.ok)
    return (
      <span className="err" title={res.error}>
        error
      </span>
    );
  const v = res.value;
  if (v === null) return <span className="muted">undefined</span>;
  return <span className="num">{formatComplex(v)}</span>;
}

function Code({ res }: { res?: SafeResult<string> }) {
  if (!res) return <span className="muted">—</span>;
  if (!res.ok)
    return (
      <span className="err" title={res.error}>
        error
      </span>
    );
  return <code>{res.value}</code>;
}

/** A rendered-math + text column, used for simplified / derivative forms. */
function FormCol({
  title,
  latex,
  text,
}: {
  title: string;
  latex?: SafeResult<string>;
  text?: SafeResult<string>;
}) {
  return (
    <div className="col">
      <h3>{title}</h3>
      <div className="rendered">
        {latex?.ok ? (
          <Katex tex={latex.value} display />
        ) : latex && !latex.ok ? (
          <span className="err" title={latex.error}>
            error: {latex.error}
          </span>
        ) : (
          <span className="muted">—</span>
        )}
      </div>
      <div className="kv">
        <dt>text</dt>
        <dd>
          <Code res={text} />
        </dd>
      </div>
    </div>
  );
}

function agreeBadge(ok: boolean) {
  return ok ? (
    <span className="badge ok">agree ✓</span>
  ) : (
    <span className="badge diff">differ ✗</span>
  );
}

/** One engine's parse/derivative column. */
function EngineColumn({ title, res }: { title: string; res: Analysis }) {
  if (res.parseError)
    return (
      <div className="col">
        <h3>{title}</h3>
        <p className="err">parse error: {res.parseError}</p>
      </div>
    );
  return (
    <div className="col">
      <h3>{title}</h3>
      <div className="rendered">
        {res.latex?.ok ? (
          <Katex tex={res.latex.value} display />
        ) : (
          <span className="muted">—</span>
        )}
      </div>
      <dl className="kv">
        <dt>text</dt>
        <dd>
          <Code res={res.text} />
        </dd>
        <dt>latex</dt>
        <dd>
          <Code res={res.latex} />
        </dd>
        <dt>variables</dt>
        <dd>
          {res.variables?.ok ? (
            res.variables.value.length ? (
              res.variables.value.map((v) => (
                <span key={v} className="chip">
                  {v}
                </span>
              ))
            ) : (
              <span className="muted">none (constant)</span>
            )
          ) : (
            <span className="err">error</span>
          )}
        </dd>
      </dl>
      <details open>
        <summary>parse tree</summary>
        {res.tree?.ok ? (
          <div className="tree-box">
            <Tree value={res.tree.value} />
          </div>
        ) : (
          <span className="err">error</span>
        )}
      </details>
    </div>
  );
}

/* ------------------------------- app ------------------------------- */

export default function App() {
  const [engines, setEngines] = useState<Engines | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [input, setInput] = useState("x^3 + 2x - 1");
  const [syntax, setSyntax] = useState<Syntax>("text");
  const [diffVar, setDiffVar] = useState("x");
  const [rawBindings, setRawBindings] = useState<Record<string, string>>({
    x: "2",
    y: "3",
  });
  const [assumptions, setAssumptions] = useState<string[]>([]);
  const [simplifyDeriv, setSimplifyDeriv] = useState(false);

  useEffect(() => {
    loadEngines().then(setEngines, (e: unknown) =>
      setLoadError(e instanceof Error ? e.message : String(e)),
    );
  }, []);

  // Bindings are expression strings (may be complex, e.g. "i" or "2+3i"). Every
  // non-empty entry is substituted before evaluation; extra (unused) variables
  // are harmless in both engines.
  const bindings = useMemo<Record<string, string>>(() => {
    const out: Record<string, string> = {};
    for (const [k, v] of Object.entries(rawBindings)) {
      if (v.trim() !== "") out[k] = v;
    }
    return out;
  }, [rawBindings]);

  // Non-empty, trimmed assumption strings (e.g. "x > 0").
  const activeAssumptions = useMemo(
    () => assumptions.map((s) => s.trim()).filter(Boolean),
    [assumptions],
  );

  // Validity marker per assumption string, memoized so we don't re-parse on
  // every render. Marked invalid only when BOTH engines reject it (a string
  // may parse in one implementation but not the other). Rust handles freed.
  const assumptionValidity = useMemo(() => {
    const m = new Map<string, boolean>();
    if (!engines) return m;
    for (const a of assumptions) {
      if (m.has(a)) continue;
      const t = a.trim();
      if (t === "") {
        m.set(a, true);
        continue;
      }
      const jsOk = safe(() => engines.js.parse(t, "text")).ok;
      const rustOk =
        jsOk ||
        safe(() => {
          const h = engines.rust.parse(t, "text");
          engines.rust.free(h);
        }).ok;
      m.set(a, jsOk || rustOk);
    }
    return m;
  }, [engines, assumptions]);

  // The analysis pipeline (2× parse/simplify/derivative/evaluate) is heavy —
  // the JS library's simplify in particular can be slow on intermediate input.
  // Deferring the params keeps typing responsive: React re-renders the inputs
  // urgently with the previous analysis, then recomputes at deferred priority,
  // coalescing rapid keystrokes into one analysis of the latest value.
  const params = useMemo<EngineParams>(
    () => ({
      input,
      syntax,
      diffVar,
      bindings,
      assumptions: activeAssumptions,
      simplifyDeriv,
    }),
    [input, syntax, diffVar, bindings, activeAssumptions, simplifyDeriv],
  );
  const deferredParams = useDeferredValue(params);
  const isStale = deferredParams !== params;

  const analysis = useMemo(() => {
    if (!engines) return null;
    return {
      js: analyze(engines.js, deferredParams),
      rust: analyze(engines.rust, deferredParams),
    };
  }, [engines, deferredParams]);

  // Detected variables (union of both engines) drive the substitution inputs.
  const variables = useMemo(() => {
    const s = new Set<string>();
    for (const a of [analysis?.js, analysis?.rust]) {
      const vars = a?.variables;
      if (vars && vars.ok) vars.value.forEach((v) => s.add(v));
    }
    return [...s];
  }, [analysis]);

  if (loadError)
    return (
      <div className="app">
        <h1>Failed to load engines</h1>
        <pre className="err">{loadError}</pre>
      </div>
    );
  if (!engines || !analysis)
    return (
      <div className="app">
        <h1>math-expressions playground</h1>
        <p>Loading Rust WASM…</p>
      </div>
    );

  const { js, rust } = analysis;
  const bothParsed = !js.parseError && !rust.parseError;
  const jsTree = okValue(js.tree);
  const rustTree = okValue(rust.tree);
  const treesMatch =
    bothParsed &&
    jsTree !== undefined &&
    rustTree !== undefined &&
    deepEqual(jsTree, rustTree);
  const jsSimp = okValue(js.simpTree);
  const rustSimp = okValue(rust.simpTree);
  const simpMatch =
    bothParsed &&
    jsSimp !== undefined &&
    rustSimp !== undefined &&
    deepEqual(jsSimp, rustSimp);

  const evalAgrees = (
    a?: SafeResult<Complex | null>,
    b?: SafeResult<Complex | null>,
  ): boolean => {
    if (!a?.ok || !b?.ok) return false;
    const va = a.value;
    const vb = b.value;
    if (va === null && vb === null) return true;
    if (va && vb) {
      const scale = Math.max(1, Math.abs(va.re), Math.abs(va.im));
      return (
        Math.abs(va.re - vb.re) <= 1e-7 * scale &&
        Math.abs(va.im - vb.im) <= 1e-7 * scale
      );
    }
    return false;
  };

  return (
    <div className="app">
      <header>
        <h1>math-expressions playground</h1>
        <p className="sub">
          Parse, evaluate, and differentiate — comparing the <b>Rust (WASM)</b>{" "}
          and <b>JavaScript</b> implementations side by side.
        </p>
      </header>

      {/* ---- input ---- */}
      <section className="card">
        <label className="field">
          <span>Expression</span>
          <input
            className="expr-input"
            value={input}
            spellCheck={false}
            onChange={(e) => setInput(e.target.value)}
            placeholder="type a math expression…"
          />
        </label>
        <div className="row">
          <div className="radios">
            <label>
              <input
                type="radio"
                checked={syntax === "text"}
                onChange={() => setSyntax("text")}
              />{" "}
              text
            </label>
            <label>
              <input
                type="radio"
                checked={syntax === "latex"}
                onChange={() => setSyntax("latex")}
              />{" "}
              LaTeX
            </label>
          </div>
          <div className="examples">
            {EXAMPLES.map((ex) => (
              <button
                key={ex.label}
                className="chip-btn"
                onClick={() => {
                  setInput(ex.input);
                  setSyntax(ex.syntax);
                }}
              >
                {ex.label}
              </button>
            ))}
          </div>
        </div>
      </section>

      {/* ---- assumptions ---- */}
      <section className="card">
        <div className="section-head">
          <h2>Assumptions</h2>
          <span className="muted assume-hint">
            refine the simplified result — e.g. <code>sqrt(x^2)</code> →{" "}
            <code>x</code> when <code>x&gt;0</code>
          </span>
        </div>
        <div className="assumptions">
          {assumptions.map((a, i) => {
            const valid = assumptionValidity.get(a) ?? true;
            return (
              <div key={i} className="assumption-row">
                <input
                  className={"expr-mini" + (valid ? "" : " invalid")}
                  value={a}
                  spellCheck={false}
                  placeholder="e.g. x > 0"
                  title={valid ? "" : "does not parse — ignored"}
                  onChange={(e) =>
                    setAssumptions((xs) =>
                      xs.map((x, j) => (j === i ? e.target.value : x)),
                    )
                  }
                />
                <button
                  className="x-btn"
                  title="remove"
                  onClick={() =>
                    setAssumptions((xs) => xs.filter((_, j) => j !== i))
                  }
                >
                  ×
                </button>
              </div>
            );
          })}
          <button
            className="chip-btn"
            onClick={() => setAssumptions((xs) => [...xs, ""])}
          >
            + add
          </button>
        </div>
        <div className="examples">
          {[
            "x > 0",
            "x >= 0",
            "x != 0",
            "x < 0",
            "x elementof Z",
            "x elementof R",
          ].map((a) => (
            <button
              key={a}
              className="chip-btn"
              onClick={() => setAssumptions((xs) => [...xs, a])}
            >
              {a}
            </button>
          ))}
        </div>
      </section>

      {/* Results dim briefly while a deferred re-analysis is pending. */}
      <div className={isStale ? "stale" : undefined}>
        {/* ---- parsed ---- */}
        <section className="card">
          <div className="section-head">
            <h2>Parsed expression</h2>
            {bothParsed && (
              <span title="Do both engines produce identical parse trees?">
                trees {agreeBadge(treesMatch)}
              </span>
            )}
          </div>
          <div className="cols">
            <EngineColumn title="JavaScript" res={js} />
            <EngineColumn title="Rust (WASM)" res={rust} />
          </div>
        </section>

        {/* ---- simplify ---- */}
        <section className="card">
          <div className="section-head">
            <h2>
              Simplified
              {activeAssumptions.length > 0 && (
                <span className="muted">
                  {" "}
                  · assuming {activeAssumptions.join(", ")}
                </span>
              )}
            </h2>
            {bothParsed && (
              <span title="Do both engines simplify to identical trees?">
                trees {agreeBadge(simpMatch)}
              </span>
            )}
          </div>
          <div className="cols">
            <FormCol
              title="JavaScript"
              latex={js.simpLatex}
              text={js.simpText}
            />
            <FormCol
              title="Rust (WASM)"
              latex={rust.simpLatex}
              text={rust.simpText}
            />
          </div>
        </section>

        {/* ---- evaluate ---- */}
        <section className="card">
          <div className="section-head">
            <h2>Evaluate</h2>
            <span>result {agreeBadge(evalAgrees(js.evalF, rust.evalF))}</span>
          </div>
          {variables.length === 0 ? (
            <p className="muted">
              No free variables — the expression is constant.
            </p>
          ) : (
            <>
              <div className="bindings">
                {variables.map((v) => (
                  <label key={v} className="binding">
                    <span>{v} =</span>
                    <input
                      type="text"
                      spellCheck={false}
                      value={rawBindings[v] ?? ""}
                      onChange={(e) =>
                        setRawBindings((b) => ({ ...b, [v]: e.target.value }))
                      }
                    />
                  </label>
                ))}
              </div>
              <p className="hint muted">
                Values are expressions — use <code>i</code> for complex, e.g.{" "}
                <code>2+3i</code>, <code>i</code>, or <code>pi/4</code>.
              </p>
            </>
          )}
          <table className="results">
            <thead>
              <tr>
                <th></th>
                <th>JavaScript</th>
                <th>Rust (WASM)</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td className="rowlabel">
                  f(<span className="muted">…</span>)
                </td>
                <td>
                  <EvalValue res={js.evalF} />
                </td>
                <td>
                  <EvalValue res={rust.evalF} />
                </td>
              </tr>
            </tbody>
          </table>
        </section>

        {/* ---- derivative ---- */}
        <section className="card">
          <div className="section-head">
            <h2>Derivative</h2>
            <div className="deriv-controls">
              <label
                className="toggle"
                title="Reduce the derivative with the playground simplifier, under the current assumptions"
              >
                <input
                  type="checkbox"
                  checked={simplifyDeriv}
                  onChange={(e) => setSimplifyDeriv(e.target.checked)}
                />
                simplify
                {activeAssumptions.length > 0 ? " (under assumptions)" : ""}
              </label>
              <label className="diffvar">
                d/d
                <input
                  list="var-list"
                  value={diffVar}
                  spellCheck={false}
                  onChange={(e) => setDiffVar(e.target.value)}
                />
                <datalist id="var-list">
                  {variables.map((v) => (
                    <option key={v} value={v} />
                  ))}
                </datalist>
              </label>
            </div>
          </div>
          <div className="cols">
            <FormCol title="JavaScript" latex={js.derLatex} text={js.derText} />
            <FormCol
              title="Rust (WASM)"
              latex={rust.derLatex}
              text={rust.derText}
            />
          </div>
          <table className="results">
            <thead>
              <tr>
                <th></th>
                <th>JavaScript</th>
                <th>Rust (WASM)</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td className="rowlabel">f′({diffVar}) at bindings</td>
                <td>
                  <EvalValue res={js.evalDer} />
                </td>
                <td>
                  <EvalValue res={rust.evalDer} />
                </td>
                <td>{agreeBadge(evalAgrees(js.evalDer, rust.evalDer))}</td>
              </tr>
            </tbody>
          </table>
        </section>
      </div>

      <footer>
        <p className="muted">
          JS: <code>build/math-expressions.js</code> · Rust:{" "}
          <code>math-expressions-rs</code> → wasm. The wasm is rebuilt
          automatically (via wireit) when the Rust sources change.
        </p>
      </footer>
    </div>
  );
}

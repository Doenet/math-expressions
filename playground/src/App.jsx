import { useEffect, useMemo, useState } from "react";
import { loadEngines } from "./engines.js";
import Katex from "./components/Katex.jsx";
import Tree from "./components/Tree.jsx";

/* ----------------------------- helpers ----------------------------- */

/** Run `fn`, capturing exceptions as a tagged result. */
function safe(fn) {
  try {
    return { ok: true, value: fn() };
  } catch (e) {
    return { ok: false, error: e?.message ?? String(e) };
  }
}

function formatFloat(v) {
  if (Number.isInteger(v)) return String(v);
  return String(Number(v.toPrecision(12)));
}

/** Format a `{ re, im }` value, collapsing negligible parts (a + b i / b i / a). */
function formatComplex({ re, im }) {
  const tol = 1e-9 * Math.max(1, Math.abs(re), Math.abs(im));
  const imZero = Math.abs(im) <= tol;
  const reZero = Math.abs(re) <= tol;
  if (imZero) return formatFloat(re);
  const mag = formatFloat(Math.abs(im));
  const imPart = mag === "1" ? "i" : `${mag} i`;
  if (reZero) return (im < 0 ? "−" : "") + imPart;
  return `${formatFloat(re)} ${im < 0 ? "−" : "+"} ${imPart}`;
}

function deepEqual(a, b) {
  if (Array.isArray(a) && Array.isArray(b)) {
    return a.length === b.length && a.every((x, i) => deepEqual(x, b[i]));
  }
  return a === b;
}

const EXAMPLES = [
  { label: "sin²+cos²", input: "sin^2(x) + cos^2(x)", syntax: "text" },
  { label: "cubic", input: "x^3 + 2x - 1", syntax: "text" },
  { label: "e^(xy)", input: "e^(x y)", syntax: "text" },
  { label: "rational", input: "(x^2 - 1)/(x - 1)", syntax: "text" },
  { label: "norm", input: "sqrt(x^2 + y^2)", syntax: "text" },
  { label: "LaTeX frac", input: "\\frac{x}{y} + \\sqrt{x}", syntax: "latex" },
];

/** Analyse one expression with one engine; every step is individually guarded. */
function analyze(
  engine,
  { input, syntax, diffVar, bindings, assumptions, simplifyDeriv },
) {
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
  // the handles are still live.
  const result = {
    tree: safe(() => engine.tree(h)),
    text: safe(() => engine.toText(h)),
    latex: safe(() => engine.toLatex(h)),
    variables: safe(() => engine.variables(h)),
    evalF: safe(() => engine.evaluate(h, bindings)),
    simpTree: simp.ok ? safe(() => engine.tree(simp.value)) : null,
    simpText: simp.ok ? safe(() => engine.toText(simp.value)) : null,
    simpLatex: simp.ok ? safe(() => engine.toLatex(simp.value)) : null,
    derText: derShown.ok ? safe(() => engine.toText(derShown.value)) : null,
    derLatex: derShown.ok ? safe(() => engine.toLatex(derShown.value)) : null,
    evalDer: der.ok ? safe(() => engine.evaluate(der.value, bindings)) : null,
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

/** Render an evaluation result (`safe` wrapping `{ re, im } | null`). */
function EvalValue({ res }) {
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

function Code({ res }) {
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
function FormCol({ title, latex, text }) {
  return (
    <div className="col">
      <h3>{title}</h3>
      <div className="rendered">
        {latex?.ok ? (
          <Katex tex={latex.value} display />
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

function agreeBadge(ok) {
  return ok ? (
    <span className="badge ok">agree ✓</span>
  ) : (
    <span className="badge diff">differ ✗</span>
  );
}

/** One engine's parse/derivative column. */
function EngineColumn({ title, res }) {
  if (!res)
    return (
      <div className="col">
        <h3>{title}</h3>
        <p className="muted">loading…</p>
      </div>
    );
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
  const [engines, setEngines] = useState(null);
  const [loadError, setLoadError] = useState(null);
  const [input, setInput] = useState("x^3 + 2x - 1");
  const [syntax, setSyntax] = useState("text");
  const [diffVar, setDiffVar] = useState("x");
  const [rawBindings, setRawBindings] = useState({ x: "2", y: "3" });
  const [assumptions, setAssumptions] = useState([]);
  const [simplifyDeriv, setSimplifyDeriv] = useState(false);

  useEffect(() => {
    loadEngines().then(setEngines, (e) =>
      setLoadError(e?.message ?? String(e)),
    );
  }, []);

  // Bindings are expression strings (may be complex, e.g. "i" or "2+3i"). Every
  // non-empty entry is substituted before evaluation; extra (unused) variables
  // are harmless in both engines.
  const bindings = useMemo(() => {
    const out = {};
    for (const [k, v] of Object.entries(rawBindings)) {
      if (String(v).trim() !== "") out[k] = v;
    }
    return out;
  }, [rawBindings]);

  // Non-empty, trimmed assumption strings (e.g. "x > 0").
  const activeAssumptions = useMemo(
    () => assumptions.map((s) => s.trim()).filter(Boolean),
    [assumptions],
  );

  const analysis = useMemo(() => {
    if (!engines) return null;
    const params = {
      input,
      syntax,
      diffVar,
      bindings,
      assumptions: activeAssumptions,
      simplifyDeriv,
    };
    return {
      js: analyze(engines.js, params),
      rust: analyze(engines.rust, params),
    };
  }, [
    engines,
    input,
    syntax,
    diffVar,
    bindings,
    activeAssumptions,
    simplifyDeriv,
  ]);

  // Detected variables (union of both engines) drive the substitution inputs.
  const variables = useMemo(() => {
    const s = new Set();
    for (const a of [analysis?.js, analysis?.rust]) {
      if (a?.variables?.ok) a.variables.value.forEach((v) => s.add(v));
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
  const treesMatch =
    bothParsed &&
    js.tree?.ok &&
    rust.tree?.ok &&
    deepEqual(js.tree.value, rust.tree.value);
  const simpMatch =
    bothParsed &&
    js.simpTree?.ok &&
    rust.simpTree?.ok &&
    deepEqual(js.simpTree.value, rust.simpTree.value);

  const evalAgrees = (a, b) => {
    if (!a?.ok || !b?.ok) return false;
    const va = a.value,
      vb = b.value;
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
            const valid =
              a.trim() === "" || safe(() => engines.js.parse(a, "text")).ok;
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
          <FormCol title="JavaScript" latex={js.simpLatex} text={js.simpText} />
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

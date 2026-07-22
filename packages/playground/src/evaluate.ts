// The chain evaluator. It folds a `ParsedChain` over each engine independently,
// dispatching every step through the operation registry, and pairs the two
// per-step results for the UI. Every wasm handle produced on the Rust side —
// the source, each intermediate, expression-typed args, substitution
// intermediates — is freed deterministically, including on the error path,
// mirroring the discipline the original playground used in `analyze()`.

import type { EngineAdapter } from "./engines";
import { BASE_VAR, literalToValue } from "./chain";
import { REGISTRY_BY_ID } from "./registry";
import {
  complexAgrees,
  deepEqual,
  safe,
  sameStringSet,
} from "./util";
import type {
  Displayable,
  EngineCtx,
  EngineOp,
  JsExpr,
  NativeResult,
  ParsedChain,
  RustExpr,
  SafeResult,
  StepResult,
  Syntax,
  Tree,
} from "./types";

/** The equation from the first box, referenced in a chain as `expr`. */
export interface BaseExpr {
  text: string;
  syntax: Syntax;
}

/** Lenient `Tree` check for `fromAst(...)` literals — rejects plain objects. */
function isTree(v: unknown): v is Tree {
  if (typeof v === "number" || typeof v === "string" || typeof v === "boolean")
    return true;
  if (Array.isArray(v)) return v.every(isTree);
  return false;
}

/** Build the source handle for one engine from the chain's source. */
function sourceHandle<H>(
  adapter: EngineAdapter<H>,
  source: ParsedChain["source"],
  base: BaseExpr,
): H {
  // Bare variable reference — the equation from the first box.
  if (source.kind === "var") {
    if (source.name !== BASE_VAR)
      throw new Error(
        `unknown variable "${source.name}" — use "${BASE_VAR}" or a source like parse("…")`,
      );
    if (base.text.trim() === "")
      throw new Error(`the ${BASE_VAR} box is empty`);
    return base.syntax === "latex"
      ? adapter.fromLatex(base.text)
      : adapter.fromText(base.text);
  }
  const { kind, arg } = source;
  if (kind === "fromAst") {
    const tree = literalToValue(arg);
    if (!isTree(tree))
      throw new Error(
        "fromAst(...) needs a Tree literal (number, string, boolean, or array)",
      );
    return adapter.fromAst(tree);
  }
  if (arg.kind !== "string")
    throw new Error(`${kind}(...) needs a string argument`);
  return kind === "fromLatex"
    ? adapter.fromLatex(arg.value)
    : adapter.fromText(arg.value);
}

/** Extract an Expression handle's display data (does NOT free the handle). */
function extractExpr<H>(adapter: EngineAdapter<H>, h: H): Displayable {
  return {
    kind: "expression",
    text: adapter.toText(h),
    latex: adapter.toLatex(h),
    tree: adapter.treeOf(h),
  };
}

/** Turn a native result into a handle-free Displayable (reads handles first). */
function extract<H>(
  adapter: EngineAdapter<H>,
  native: NativeResult<H>,
  stringRender?: "text" | "latex",
): Displayable {
  switch (native.kind) {
    case "expression":
      return extractExpr(adapter, native.handle);
    case "maybeExpression":
      return native.handle
        ? extractExpr(adapter, native.handle)
        : { kind: "none" };
    case "string":
      return {
        kind: "string",
        value: native.value,
        render: native.render ?? stringRender ?? "text",
      };
    case "stringList":
      return { kind: "stringList", value: native.value };
    case "boolean":
      return { kind: "boolean", value: native.value };
    case "number":
      return { kind: "number", value: native.value };
    case "complex":
      return { kind: "complex", value: native.value };
    case "tree":
      return { kind: "tree", value: native.value };
  }
}

/**
 * Fold the chain over one engine, returning exactly `1 + steps.length` results
 * (index 0 is the source). Handles are freed deterministically throughout.
 */
function runEngine<H>(
  adapter: EngineAdapter<H>,
  tag: "js" | "rust",
  chain: ParsedChain,
  base: BaseExpr,
): SafeResult<Displayable>[] {
  const results: SafeResult<Displayable>[] = [];
  const total = 1 + chain.steps.length;
  const pushEnded = () => results.push({ ok: true, value: { kind: "ended" } });

  let recv: H | null = null;
  let ended = false;

  // ---- source (parse handle + its display, freeing the handle if display throws) ----
  const srcRes = safe(() => {
    const h = sourceHandle(adapter, chain.source, base);
    try {
      return { h, disp: extractExpr(adapter, h) };
    } catch (e) {
      adapter.free(h);
      throw e;
    }
  });
  if (!srcRes.ok) {
    results.push(srcRes);
    while (results.length < total) pushEnded();
    return results;
  }
  recv = srcRes.value.h;
  results.push({ ok: true, value: srcRes.value.disp });

  // ---- steps ----
  for (const step of chain.steps) {
    if (ended || recv === null) {
      pushEnded();
      continue;
    }
    const entry = REGISTRY_BY_ID.get(step.method);
    if (!entry) {
      results.push({ ok: false, error: `unknown method .${step.method}()` });
      ended = true;
      continue;
    }
    const engineOp = (tag === "js" ? entry.js : entry.rust) as
      | EngineOp<H>
      | null;
    if (engineOp === null) {
      const reason =
        tag === "js" ? entry.unsupportedReason?.js : entry.unsupportedReason?.rust;
      results.push({ ok: true, value: { kind: "unsupported", reason } });
      ended = true; // this engine cannot continue past an op it does not implement
      continue;
    }

    // recv is non-null here (guarded above); the cast also breaks a circular
    // inference with the closure below, which reassigns `recv`.
    const receiver = recv as H;
    const out = safe((): Displayable => {
      const temps: H[] = [];
      const ctx: EngineCtx<H> = {
        parseExpr: adapter.parseExpr,
        track: (h) => {
          temps.push(h);
          return h;
        },
        toText: adapter.toText,
        toLatex: adapter.toLatex,
      };
      let produced: H | null = null;
      try {
        const native = engineOp.run(receiver, step.args, ctx);
        if (native.kind === "expression") {
          produced = native.handle;
          temps.push(produced);
        } else if (native.kind === "maybeExpression" && native.handle) {
          produced = native.handle;
          temps.push(produced);
        }
        // Read all display data BEFORE any free.
        const disp = extract(adapter, native, entry.stringRender);
        if (produced !== null) {
          const p = produced;
          // Promote: remove the new receiver from temps so `finally` won't free
          // it. Guard indexOf so a not-found value can never splice off the tail.
          const idx = temps.indexOf(p);
          if (idx >= 0) temps.splice(idx, 1);
          if (receiver !== p) adapter.free(receiver);
          recv = p;
        } else {
          adapter.free(receiver);
          recv = null;
          ended = true;
        }
        return disp;
      } finally {
        for (const t of temps) adapter.free(t);
      }
    });
    results.push(out);
    if (!out.ok) {
      // The step threw before swapping receivers, so `receiver` was never freed
      // (any produced/arg handles were freed by the finally). Free it once.
      if (recv === receiver) {
        adapter.free(receiver);
        recv = null;
      }
      ended = true;
    }
  }

  if (recv !== null) adapter.free(recv);
  return results;
}

/** Compare two step displayables for the agree/differ badge. */
function agreeOf(
  a: SafeResult<Displayable>,
  b: SafeResult<Displayable>,
): boolean | undefined {
  if (!a.ok || !b.ok) return undefined;
  const x = a.value;
  const y = b.value;
  if (x.kind !== y.kind) return undefined;
  switch (x.kind) {
    case "expression":
      return deepEqual(x.tree, (y as typeof x).tree);
    case "tree":
      return deepEqual(x.value, (y as typeof x).value);
    case "stringList":
      return sameStringSet(x.value, (y as typeof x).value);
    case "string":
      return x.value === (y as typeof x).value;
    case "boolean":
      return x.value === (y as typeof x).value;
    case "number": {
      const yv = (y as typeof x).value;
      return complexAgrees(
        x.value === null ? null : { re: x.value, im: 0 },
        yv === null ? null : { re: yv, im: 0 },
      );
    }
    case "complex":
      return complexAgrees(x.value, (y as typeof x).value);
    default:
      return undefined; // none / unsupported / ended
  }
}

/** Run a parsed chain against both engines and pair the per-step results. */
export function evaluateChain(
  js: EngineAdapter<JsExpr>,
  rust: EngineAdapter<RustExpr>,
  chain: ParsedChain,
  base: BaseExpr,
): StepResult[] {
  const jsResults = runEngine(js, "js", chain, base);
  const rustResults = runEngine(rust, "rust", chain, base);
  const total = 1 + chain.steps.length;
  const steps: StepResult[] = [];
  for (let i = 0; i < total; i++) {
    const entry =
      i === 0 ? undefined : REGISTRY_BY_ID.get(chain.steps[i - 1].method);
    const label =
      i === 0
        ? chain.source.kind === "var"
          ? chain.source.name
          : chain.source.kind
        : entry?.display ?? chain.steps[i - 1].method;
    const jsR = jsResults[i];
    const rustR = rustResults[i];
    steps.push({
      label,
      call: { js: entry?.js?.call, rust: entry?.rust?.call },
      jsResult: jsR,
      rustResult: rustR,
      agree: agreeOf(jsR, rustR),
    });
  }
  return steps;
}

// Runtime reflection of the Rust (WASM) API surface into playground operations.
//
// The wasm-bindgen build emits `math_expressions_wasm.d.ts`, an exact, richly
// typed listing of every exported `Expression` method — regenerated on every
// `build:wasm`. We serve that file alongside the wasm (see vite.config.ts +
// engines.ts) and parse it here, so the palette's "Other" category always
// reflects the *live* API: add a `#[wasm_bindgen]` method in Rust, rebuild, and
// a button for it appears automatically. No hand-maintained list, no drift, no
// coverage test to keep green.
//
// Only `Expression` instance methods are surfaced — they are the receiver-chain
// shape the palette models (`expr.method(...)`). Free functions (`parse_*`,
// `gcd`, `solve_ode`, …) and the standalone `Assumptions` / `OdeSolution`
// classes are not methods on an expression, so they have no place in a chain
// and are intentionally excluded.

import { CURATED_RUST_METHODS, REGISTRY_BY_ID } from "./registry";
import type {
  ArgKind,
  ArgSpec,
  EngineCtx,
  EngineOp,
  JsExpr,
  Literal,
  NativeResult,
  OpEntry,
  ReturnKind,
  RustExpr,
} from "./types";

/* ------------------------------ arg readers ----------------------------- */
// Mirrors of the readers in registry.ts, kept local so this module is a
// self-contained reflection layer over whatever the .d.ts describes.

function needStr(lit: Literal | undefined, name: string): string {
  if (!lit) throw new Error(`missing argument "${name}"`);
  if (lit.kind !== "string")
    throw new Error(`argument "${name}" must be a string`);
  return lit.value;
}
function needNum(lit: Literal | undefined, name: string): number {
  if (!lit) throw new Error(`missing argument "${name}"`);
  if (lit.kind !== "number")
    throw new Error(`argument "${name}" must be a number`);
  return lit.value;
}
function needBool(lit: Literal | undefined, name: string): boolean {
  if (!lit) throw new Error(`missing argument "${name}"`);
  if (lit.kind !== "boolean")
    throw new Error(`argument "${name}" must be a boolean`);
  return lit.value;
}
function optStringArray(lit: Literal | undefined): string[] {
  if (!lit) return [];
  if (lit.kind !== "array") throw new Error("expected an array of strings");
  return lit.items.map((it) => {
    if (it.kind !== "string") throw new Error("array items must be strings");
    return it.value;
  });
}

/* --------------------------- .d.ts type parsing -------------------------- */

/** A parsed `name(params): ret` signature from the `Expression` class body. */
interface RawMethod {
  name: string;
  params: { name: string; optional: boolean; type: string }[];
  ret: string;
}

/** Strip `/* *​/` block and `//` line comments so brace-matching is reliable. */
function stripComments(src: string): string {
  return src
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/^[ \t]*\/\/.*$/gm, "");
}

/** Extract the `{ … }` body of `export class <name>` (balanced braces). */
function classBody(src: string, name: string): string | null {
  const head = src.indexOf(`export class ${name}`);
  if (head < 0) return null;
  const open = src.indexOf("{", head);
  if (open < 0) return null;
  let depth = 0;
  for (let i = open; i < src.length; i++) {
    if (src[i] === "{") depth++;
    else if (src[i] === "}" && --depth === 0) return src.slice(open + 1, i);
  }
  return null;
}

/** Split a parameter list on top-level commas (the surface has no nested ones). */
function splitParams(params: string): string[] {
  return params.trim() === "" ? [] : params.split(",").map((p) => p.trim());
}

/** Parse the (comment-free) `Expression` class body into method signatures. */
function parseExpressionMethods(dts: string): RawMethod[] {
  const body = classBody(stripComments(dts), "Expression");
  if (!body) return [];
  const methods: RawMethod[] = [];
  // Each member is a single line: `name(<params>): <ret>;`. `constructor`,
  // `free`, and `[Symbol.dispose]` don't match `<ident>(` after a private/`[`.
  const sig = /^\s*([A-Za-z_]\w*)\s*\(([^)]*)\)\s*:\s*([^;]+);\s*$/;
  for (const line of body.split("\n")) {
    const m = sig.exec(line);
    if (!m) continue;
    const [, name, rawParams, ret] = m;
    if (name === "constructor" || name === "free") continue;
    const params = splitParams(rawParams).map((p) => {
      const pm = /^(\w+)(\?)?\s*:\s*(.+)$/.exec(p);
      if (!pm) return { name: p, optional: false, type: "unknown" };
      return { name: pm[1], optional: pm[2] === "?", type: pm[3].trim() };
    });
    methods.push({ name, params, ret: ret.trim() });
  }
  return methods;
}

/* ----------------------- TS type → playground kinds ---------------------- */

const VAR_PARAM = /^_?(var|variable)$/;

/** Map a parameter's TS type to an {@link ArgKind}, or null if unrepresentable. */
function argKindOf(paramName: string, type: string): ArgKind | null {
  const t = type.replace(/\s*\|\s*(undefined|null)/g, "").trim();
  if (t === "Expression") return "expression";
  if (t === "string") return VAR_PARAM.test(paramName) ? "variable" : "string";
  if (t === "string[]" || t === "Array<string>") return "stringArray";
  if (t === "number") return "number";
  if (t === "boolean") return "boolean";
  return null; // Float64Array, Int32Array, Function, DecimalFormat, …
}

/* --------------------- native-result wrap by return type ----------------- */

/** How an op's return value is packaged for the UI, derived from the TS return type. */
interface ReturnPlan {
  returns: ReturnKind;
  stringRender?: "text" | "latex";
  wrap: (raw: unknown) => NativeResult<unknown>;
}

/** Map a TS return type to a {@link ReturnPlan}, or null if unrepresentable. */
function returnPlanOf(ret: string): ReturnPlan | null {
  const nullable = /\|\s*(undefined|null)/.test(ret);
  const base = ret.replace(/\s*\|\s*(undefined|null)/g, "").trim();
  switch (base) {
    case "Expression":
      return nullable
        ? {
            returns: "maybeExpression",
            wrap: (r) => ({ kind: "maybeExpression", handle: r ?? null }),
          }
        : { returns: "expression", wrap: (r) => ({ kind: "expression", handle: r }) };
    case "string":
      // `string | undefined` is the "not decidable / no result" signal.
      return {
        returns: "string",
        stringRender: "text",
        wrap: (r) => ({
          kind: "string",
          value: r == null ? "undefined" : (r as string),
          render: "text",
        }),
      };
    case "string[]":
    case "Array<string>":
      return {
        returns: "stringList",
        wrap: (r) => ({ kind: "stringList", value: r ? Array.from(r as string[]) : [] }),
      };
    case "boolean":
      // A certified predicate (`boolean | undefined`) can be undecided — render
      // it as text so the third state is visible, matching curated `is_zero`.
      return nullable
        ? {
            returns: "string",
            stringRender: "text",
            wrap: (r) => ({
              kind: "string",
              value: r == null ? "undecided" : String(r),
              render: "text",
            }),
          }
        : { returns: "boolean", wrap: (r) => ({ kind: "boolean", value: !!r }) };
    case "number":
      return {
        returns: "number",
        wrap: (r) => ({ kind: "number", value: r == null ? null : Number(r) }),
      };
    default:
      return null; // void, Expression[], Float64Array, … — not chainable here
  }
}

/* ------------------------- op assembly from a method --------------------- */

/** Placeholder inserted for an arg of each kind in the palette snippet. */
const PLACEHOLDER: Record<ArgKind, string> = {
  expression: '"x"',
  variable: '"x"',
  string: '""',
  number: "0",
  boolean: "false",
  stringArray: "[]",
  substitutionMap: '{x: "2"}',
};

/** Materialize one parsed literal into the value the wasm method expects. */
function materialize<H>(spec: ArgSpec, lit: Literal | undefined, c: EngineCtx<H>): unknown {
  switch (spec.kind) {
    case "expression":
      // Track so the evaluator frees the parsed arg handle after the step.
      return c.track(c.parseExpr(needStr(lit, spec.name)));
    case "variable":
    case "string":
      return needStr(lit, spec.name);
    case "number":
      return needNum(lit, spec.name);
    case "boolean":
      return needBool(lit, spec.name);
    case "stringArray":
      return optStringArray(lit);
    default:
      throw new Error(`arg "${spec.name}" is not supported by the playground`);
  }
}

/** Build a generic dispatcher that calls `recv[name](...args)` and wraps the result. */
function makeRun<H>(
  name: string,
  args: ArgSpec[],
  plan: ReturnPlan,
): EngineOp<H>["run"] {
  return (recv, a, c) => {
    const callArgs = args.map((spec, i) => materialize(spec, a[i], c));
    const fn = (recv as Record<string, (...xs: unknown[]) => unknown>)[name];
    if (typeof fn !== "function")
      throw new Error(`.${name}() is not available on this engine`);
    return plan.wrap(fn.apply(recv, callArgs)) as NativeResult<H>;
  };
}

/** Turn one parsed method into an {@link OpEntry}, or null if unrepresentable. */
function opFromMethod(m: RawMethod, jsHas: (name: string) => boolean): OpEntry | null {
  const plan = returnPlanOf(m.ret);
  if (!plan) return null;

  const args: ArgSpec[] = [];
  for (const p of m.params) {
    const kind = argKindOf(p.name, p.type);
    if (kind === null) {
      // Drop an unrepresentable optional arg; bail on an unrepresentable required one.
      if (p.optional) continue;
      return null;
    }
    args.push({ name: p.name.replace(/^_/, ""), kind, optional: p.optional });
  }

  const insertText = `${m.name}(${args.map((s) => PLACEHOLDER[s.kind]).join(", ")})`;
  const rust: EngineOp<RustExpr> = {
    call: `${m.name}(${args.map((s) => s.name).join(", ")})`,
    run: makeRun<RustExpr>(m.name, args, plan),
  };
  // Auto-wire the JS engine only when its Expression exposes a same-named
  // method (the Rust port mirrors JS names). Otherwise mark it JS-unsupported —
  // the palette then renders it "rust-only", exactly like curated `factor`.
  const js: EngineOp<JsExpr> | null = jsHas(m.name)
    ? { call: `${m.name}(${args.map((s) => s.name).join(", ")})`, run: makeRun<JsExpr>(m.name, args, plan) }
    : null;

  return {
    id: m.name,
    display: m.name,
    category: "Other",
    args,
    returns: plan.returns,
    stringRender: plan.stringRender,
    insertText,
    js,
    rust,
    unsupportedReason: js
      ? undefined
      : { js: "auto-generated from the Rust WASM API; the JS library has no method of this name" },
  };
}

/* ------------------------------- public API ------------------------------ */

/** Diagnostics from a {@link buildDynamicOps} run (surfaced in the console). */
export interface DynamicOpsReport {
  ops: OpEntry[];
  /** Methods present in the API but not representable as a chain op, with why. */
  skipped: { name: string; reason: string }[];
}

/**
 * Reflect the wasm `Expression` surface into palette operations for every method
 * the curated {@link REGISTRY_BY_ID} does not already cover.
 *
 * Hybrid source of truth: the `.d.ts` text supplies the *types* (arg/return
 * kinds — see the note at the top of this file), while `rustHas` reports whether
 * a method actually exists on the *live* wasm `Expression` prototype. A method is
 * only surfaced when it is in both, so a stale or hand-edited `.d.ts` declaring a
 * method the running wasm lacks can never produce a dead palette button.
 * `jsHas` likewise reports whether the canonical JS `Expression` exposes the
 * method, so shared methods light up on both engines.
 */
export function buildDynamicOpsReport(
  dts: string,
  rustHas: (name: string) => boolean,
  jsHas: (name: string) => boolean,
): DynamicOpsReport {
  const ops: OpEntry[] = [];
  const skipped: { name: string; reason: string }[] = [];
  for (const m of parseExpressionMethods(dts)) {
    if (CURATED_RUST_METHODS.has(m.name) || REGISTRY_BY_ID.has(m.name)) continue;
    if (!rustHas(m.name)) {
      skipped.push({
        name: m.name,
        reason: "declared in the .d.ts but absent from the live wasm Expression prototype",
      });
      continue;
    }
    const op = opFromMethod(m, jsHas);
    if (op) ops.push(op);
    else skipped.push({ name: m.name, reason: `signature "(${m.params.map((p) => p.type).join(", ")}) => ${m.ret}" is not chainable in the playground` });
  }
  ops.sort((a, b) => a.id.localeCompare(b.id));
  return { ops, skipped };
}

/** Convenience wrapper returning just the ops (diagnostics logged to the console). */
export function buildDynamicOps(
  dts: string,
  rustHas: (name: string) => boolean,
  jsHas: (name: string) => boolean,
): OpEntry[] {
  const { ops, skipped } = buildDynamicOpsReport(dts, rustHas, jsHas);
  if (skipped.length)
    console.info(
      `[playground] ${ops.length} wasm methods auto-added to "Other"; ` +
        `${skipped.length} not surfaced: ${skipped.map((s) => s.name).join(", ")}`,
    );
  return ops;
}

/** Collect every method name reachable on a live handle's prototype chain. */
export function collectMethodNames(handle: object): Set<string> {
  const names = new Set<string>();
  let proto: object | null = Object.getPrototypeOf(handle);
  while (proto && proto !== Object.prototype) {
    for (const n of Object.getOwnPropertyNames(proto))
      if (n !== "constructor") names.add(n);
    proto = Object.getPrototypeOf(proto);
  }
  return names;
}

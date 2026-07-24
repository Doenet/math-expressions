// Guards the invariant the playground's runtime API reflection depends on: every
// method reachable by introspecting a live wasm `Expression` is also declared in
// the generated `math_expressions_wasm.d.ts`.
//
// The playground surfaces the wasm API in its "Operations" palette by reading
// method *names* off the live prototype (introspection) and their *types* out of
// the `.d.ts` (a live object exposes names + arity, never parameter types). A
// method present on the object but missing from the `.d.ts` would therefore be
// untypable — and so invisible — there. Both artifacts come from one
// `build:wasm`, so this only fails if the committed/served `.d.ts` drifts from
// the actual wasm (a stale or hand-edited declarations file).
import { readFileSync } from "node:fs";
import { describe, expect, test } from "vitest";
import * as me from "../pkg/math_expressions_wasm.js";

me.initSync({
  module: readFileSync(new URL("../pkg/math_expressions_wasm_bg.wasm", import.meta.url)),
});

/**
 * Public method names on a live handle's prototype chain. Drops the constructor
 * and the wasm-bindgen lifecycle members (`free`, `__destroy_into_raw`, …);
 * `[Symbol.dispose]` is a symbol key and so never appears here. These are not
 * part of the public API the playground reflects.
 */
function liveMethodNames(obj: object): string[] {
  const names = new Set<string>();
  for (
    let proto = Object.getPrototypeOf(obj);
    proto && proto !== Object.prototype;
    proto = Object.getPrototypeOf(proto)
  )
    for (const n of Object.getOwnPropertyNames(proto)) names.add(n);
  return [...names].filter(
    (n) => n !== "constructor" && n !== "free" && !n.startsWith("__"),
  );
}

/** Method names declared on `export class <cls>` in a wasm-bindgen `.d.ts`. */
function dtsClassMethods(dts: string, cls: string): Set<string> {
  // Strip block comments first — the doc comments contain `{ }` (JSON examples)
  // that would break brace-matching.
  const src = dts.replace(/\/\*[\s\S]*?\*\//g, "");
  const head = src.indexOf(`export class ${cls}`);
  if (head < 0) throw new Error(`class ${cls} not found in .d.ts`);
  const open = src.indexOf("{", head);
  let depth = 0;
  let end = -1;
  for (let i = open; i < src.length; i++) {
    if (src[i] === "{") depth++;
    else if (src[i] === "}" && --depth === 0) {
      end = i;
      break;
    }
  }
  const body = src.slice(open + 1, end);
  const methods = new Set<string>();
  for (const line of body.split("\n")) {
    // A member line is `name(` at the start (after indentation). `private
    // constructor();` and `[Symbol.dispose](): void;` don't match `<ident>(`.
    const m = /^\s*([A-Za-z_]\w*)\s*\(/.exec(line);
    if (m) methods.add(m[1]);
  }
  return methods;
}

const DTS = readFileSync(
  new URL("../pkg/math_expressions_wasm.d.ts", import.meta.url),
  "utf8",
);

describe("Expression API surface: introspection ⊆ .d.ts", () => {
  test("every live Expression method is declared in the .d.ts", () => {
    const declared = dtsClassMethods(DTS, "Expression");
    const live = liveMethodNames(me.parse_text("x"));

    // Sanity: introspection actually found the surface (guards a broken probe
    // from making the subset check vacuously pass).
    expect(live.length).toBeGreaterThan(20);

    const missing = live.filter((n) => !declared.has(n));
    expect(
      missing,
      `wasm Expression methods missing from math_expressions_wasm.d.ts ` +
        `(rebuild the wasm to regenerate it): ${missing.join(", ")}`,
    ).toEqual([]);
  });
});

// Encoding an AST as the JSON the wasm `from_ast` reads.
//
// `JSON.stringify` has no representation for ±Infinity or NaN — it emits
// `null`, which `from_ast` rejects outright ("unexpected value null"). The
// wire format tags them instead, so anything handing a tree to wasm has to
// replace them on the way out. This lives on its own so the converters and
// `Expression.fromAst` tag them identically rather than one of them forgetting.

/** The tagged form of a non-finite number, or the value unchanged. */
export function tagNonFinite(value: unknown): unknown {
  if (typeof value === "number" && !Number.isFinite(value)) {
    if (Number.isNaN(value)) return { $: "NaN" };
    return { $: value > 0 ? "Inf" : "-Inf" };
  }
  return value;
}

/** `JSON.stringify` of a plain AST, with non-finite numbers tagged. */
export function astToJson(ast: unknown): string {
  return JSON.stringify(ast, (_key, value) => tagNonFinite(value));
}

/**
 * The three tags that have a JS scalar, as a null-prototype lookup so a tag
 * spelled `constructor` or `toString` cannot match an inherited property.
 *
 * `None` is deliberately absent: `{"$":"None"}` has no JS scalar to become, so
 * it stays tagged in both directions. DoenetML emits it itself and reads it
 * back unchanged.
 */
const UNTAGGED: Record<string, number> = Object.assign(Object.create(null), {
  Inf: Infinity,
  "-Inf": -Infinity,
  NaN: NaN,
});

/**
 * `JSON.parse` reviver that turns the non-finite tags back into JS scalars —
 * the inverse of [`tagNonFinite`], and the reason `.tree` reads as `Infinity`
 * rather than `{"$":"Inf"}`.
 *
 * The wire format has to stay tagged (JSON cannot hold `Infinity`), but the
 * *value* a caller sees should be the scalar legacy handed back, because that
 * is what `typeof x === "number"` and `x === -Infinity` consumers test. Going
 * back in, `astReplacer` re-tags, so a tree survives a `.tree` → `fromAst`
 * round trip unchanged.
 */
export function untagNonFinite(_key: string, value: unknown): unknown {
  if (value !== null && typeof value === "object" && !Array.isArray(value)) {
    const tag = (value as { $?: unknown }).$;
    if (typeof tag === "string" && tag in UNTAGGED) return UNTAGGED[tag];
  }
  return value;
}

/** `JSON.parse` of a wasm-produced AST, with non-finite tags decoded. */
export function jsonToAst(json: string): unknown {
  return JSON.parse(json, untagNonFinite);
}

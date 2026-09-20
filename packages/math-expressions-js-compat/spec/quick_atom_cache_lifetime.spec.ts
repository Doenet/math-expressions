// Lifetime of the handles the atom cache shares.
//
// `fromAst` hands the *same* wasm handle to every caller that asks for the same
// small integer or symbol, so a wrapper's `free()` cannot simply release it —
// the other wrappers would be left pointing at freed memory. The cache is also
// dropped wholesale once it passes `MAX_ATOMS`, which orphans handles that no
// further `fromAst` will ever hand out again; those are exactly the ones
// `free()` must release, or it is a silent no-op for every atom.
import { createRequire } from "node:module";
import { describe, expect, it } from "vitest";
import me from "../lib/math-expressions";
import { setWasmModule } from "../lib/_wasm";

/** The raw wasm pointer, or 0 once the handle has been released. */
function ptr(e: unknown): number {
  return (e as { _w?: { __wbg_ptr: number } })._w?.__wbg_ptr ?? 0;
}

/** The vendored node build — the same module `_wasm`'s fallback loads. */
function nodeWasmModule(): Parameters<typeof setWasmModule>[0] {
  const require = createRequire(import.meta.url);
  return require("../vendor/wasm/math_expressions_wasm.js") as Parameters<
    typeof setWasmModule
  >[0];
}

/** Overflow `MAX_ATOMS` (4096) so the cache is dropped wholesale. */
function sweepAtomCache() {
  for (let i = 0; i < 4200; i++) me.fromAst("sweep_" + i);
}

describe("atom cache handle lifetime", () => {
  it("shares one handle across wrappers for the same atom", () => {
    const a = me.fromAst("x");
    const b = me.fromAst("x");
    expect(a).not.toBe(b);
    expect(ptr(a)).toBe(ptr(b));
    expect(ptr(a)).not.toBe(0);
  });

  it("keeps a still-cached handle alive when one wrapper is freed", () => {
    const a = me.fromAst("keepalive_sym");
    const b = me.fromAst("keepalive_sym");
    const shared = ptr(a);
    a.free();
    // `b` is untouched, and the cache will still hand the handle out.
    expect(ptr(b)).toBe(shared);
    expect(b.toString()).toBe("keepalive_sym");
    expect(ptr(me.fromAst("keepalive_sym"))).toBe(shared);
  });

  it("releases an orphaned handle once its last wrapper is freed", () => {
    const e = me.fromAst("orphan_sym");
    const orphaned = ptr(e);
    expect(orphaned).not.toBe(0);

    // The sweep drops the table, so nothing will hand this handle out again.
    sweepAtomCache();
    const handle = (e as unknown as { _w: { __wbg_ptr: number } })._w;
    e.free();

    // Before this fix the handle survived `free()` and was left to the GC.
    expect(handle.__wbg_ptr).toBe(0);
    // The cache re-mints on the next request. (Its *address* may well be the
    // one just released — the allocator is free to reuse it, which is the
    // point of releasing — so only the handle object's identity is checked.)
    const again = me.fromAst("orphan_sym");
    expect((again as unknown as { _w: unknown })._w).not.toBe(handle);
    expect(again.toString()).toBe("orphan_sym");
    expect(orphaned).not.toBe(0);
  });

  it("does not release an orphan that another wrapper still holds", () => {
    const a = me.fromAst("aliased_sym");
    const b = me.fromAst("aliased_sym");
    const handle = (a as unknown as { _w: { __wbg_ptr: number } })._w;

    sweepAtomCache();
    a.free();

    // `a` was not the last wrapper, so `b` must still be usable — releasing
    // here is the use-after-free that a naive `ATOM_SHARED.delete` would cause.
    expect(handle.__wbg_ptr).not.toBe(0);
    expect(b.toString()).toBe("aliased_sym");

    b.free();
    expect(handle.__wbg_ptr).toBe(0);
  });

  it("stays idempotent: freeing twice is a no-op", () => {
    const e = me.fromAst("double_free_sym");
    sweepAtomCache();
    e.free();
    expect(() => e.free()).not.toThrow();
  });

  it("drops cached handles when the wasm module is swapped", () => {
    const first = me.fromAst("swap_sym");
    const handle = (first as unknown as { _w: unknown })._w;
    expect((me.fromAst("swap_sym") as unknown as { _w: unknown })._w).toBe(
      handle,
    );

    // Stand in for a browser host's `setWasmModule(webBuild)`. Injecting the
    // node build the fallback had already loaded is a different code path to
    // the same functions, so the wasm still works while `injected` changes —
    // which is what has to invalidate the table.
    setWasmModule(nodeWasmModule());

    // A stale handle would now be one the injected module rejects with
    // "expected instance of Expression"; the cache must have dropped it.
    const after = me.fromAst("swap_sym");
    expect((after as unknown as { _w: unknown })._w).not.toBe(handle);
    expect(after.toString()).toBe("swap_sym");
  });

  it("ignores a re-injection of the module already in effect", () => {
    const mod = nodeWasmModule();
    setWasmModule(mod);
    const handle = (me.fromAst("reswap_sym") as unknown as { _w: unknown })._w;
    setWasmModule(mod);
    expect((me.fromAst("reswap_sym") as unknown as { _w: unknown })._w).toBe(
      handle,
    );
  });
});

/**
 * Smoke test of the *published* ES entry point — `dist/math-expressions.js`,
 * what `exports["."]` resolves to — rather than of `lib/` the way every other
 * spec here does. It is the only check that the bundle a consumer installs
 * actually works: `lib/` passing says nothing about what Vite emitted.
 *
 * It degrades to a `todo` when `dist/` is absent, because `npm test` does not
 * depend on `npm run build`. CI's packaging job builds first, so there it runs.
 */
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const BUILD_PATH = path.resolve(__dirname, "../dist/math-expressions.js");

const buildExists = fs.existsSync(BUILD_PATH);

let ME, lib;
if (buildExists) {
  lib = await import(/* @vite-ignore */ BUILD_PATH);
  ME = lib?.default;
}

describe("ESM build", () => {
  if (!buildExists) {
    it.todo("build not found — run `npm run build` first");
    return;
  }

  it("exports a default MathExpression context", () => {
    expect(typeof ME).toBe("object");
    expect(typeof ME.fromText).toBe("function");
    expect(typeof ME.fromLatex).toBe("function");
  });

  it("exports isTree as a named export", () => {
    expect(typeof lib.isTree).toBe("function");
    expect(lib.isTree("x")).toBe(true);
    expect(lib.isTree(["+", 1, "x"])).toBe(true);
    expect(lib.isTree(null)).toBe(false);
  });

  it("exports the v3 additions as named exports", () => {
    // `dopri` and `setWasmModule` are what a host reaches for; both are
    // declared in `types/math-expressions.d.ts` and neither existed in the
    // legacy library's bundle.
    expect(typeof lib.dopri).toBe("function");
    expect(typeof lib.setWasmModule).toBe("function");
  });

  it("parses text expressions", () => {
    expect(ME.fromText("x^2 + 2*x + 1").toString()).toBe("x^2 + 2 x + 1");
  });

  it("parses LaTeX expressions", () => {
    expect(ME.fromLatex("\\frac{x+1}{2}").toString()).toBe("(x + 1)/2");
  });

  it("computes symbolic derivatives", () => {
    expect(ME.fromText("x^2").derivative("x").toString()).toBe("2 x");
    expect(ME.fromText("sin(x)").derivative("x").toString()).toBe("cos(x)");
  });

  it("tests expression equality", () => {
    expect(ME.fromText("sin^2(x) + cos^2(x)").equals(ME.fromText("1"))).toBe(
      true,
    );
    expect(ME.fromText("x^2").equals(ME.fromText("x^3"))).toBe(false);
  });

  it("converts to LaTeX", () => {
    expect(ME.fromText("x^2").toLatex()).toBe("x^{2}");
  });

  it("reaches math.js through the external import", () => {
    // `mathjs` is the one dependency left as a bare import in the bundle
    // (it is 95% of the bytes when inlined). `f()` compiles through it, so
    // this fails if the external ever stops resolving.
    expect(ME.fromText("x^2").f()({ x: 3 })).toBe(9);
  });
});

/**
 * Smoke test of the *published* UMD bundle, `dist/math-expressions_umd.js`,
 * loaded the way a browser `<script>` loads it: in a context with no `exports`
 * or `module`, so the global-assignment branch runs and the library lands on
 * `globalThis.MathExpression`.
 *
 * Two things that context does not have, and that the ES test does not need:
 *
 *   - **math.js.** It is external in both builds (95% of the bytes otherwise),
 *     and UMD resolves an external through a global. `math` is math.js's own
 *     UMD global name, so that is what the sandbox has to provide.
 *   - **a wasm module.** The node fallback in `lib/_wasm.ts` reaches for
 *     `process.getBuiltinModule`, which a bare `vm` context has not got — and
 *     the UMD format has no `import.meta.url` for `createRequire` either. That
 *     is the browser situation exactly, so the test does what a browser host
 *     does: inject through `setWasmModule` before parsing anything.
 *
 * It degrades to a `todo` when `dist/` is absent; see `build_esm.spec.ts`.
 */
import fs from "fs";
import path from "path";
import vm from "vm";
import { createRequire } from "module";
import { fileURLToPath } from "url";
import * as mathjs from "mathjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const BUILD_PATH = path.resolve(__dirname, "../dist/math-expressions_umd.js");

const buildExists = fs.existsSync(BUILD_PATH);

let ME, isTree;
if (buildExists) {
  const ctx = vm.createContext({ math: mathjs });
  vm.runInContext(fs.readFileSync(BUILD_PATH, "utf8"), ctx);
  ctx.MathExpression.setWasmModule(
    createRequire(import.meta.url)("../vendor/wasm/math_expressions_wasm.js"),
  );
  ME = ctx.MathExpression?.default;
  isTree = ctx.MathExpression?.isTree;
}

describe("UMD build", () => {
  if (!buildExists) {
    it.todo("build not found — run `npm run build` first");
    return;
  }

  it("exposes MathExpression as a browser global", () => {
    expect(typeof ME).toBe("object");
    expect(typeof ME.fromText).toBe("function");
    expect(typeof ME.fromLatex).toBe("function");
  });

  it("exposes isTree on the global", () => {
    expect(typeof isTree).toBe("function");
    expect(isTree("x")).toBe(true);
    expect(isTree(["+", 1, "x"])).toBe(true);
    expect(isTree(null)).toBe(false);
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
});

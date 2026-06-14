import fs from "fs";
import path from "path";
import vm from "vm";

const BUILD_PATH = path.resolve(__dirname, "../build/math-expressions_umd.js");

const buildExists = fs.existsSync(BUILD_PATH);

// Load the UMD bundle in a browser-like vm context (no `exports`/`module`),
// which triggers the global-assignment branch: globalThis.MathExpression = {...}
let ME, isTree;
if (buildExists) {
    const ctx = vm.createContext({});
    vm.runInContext(fs.readFileSync(BUILD_PATH, "utf8"), ctx);
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
        expect(
            ME.fromText("sin^2(x) + cos^2(x)").equals(ME.fromText("1"))
        ).toBe(true);
        expect(ME.fromText("x^2").equals(ME.fromText("x^3"))).toBe(false);
    });

    it("converts to LaTeX", () => {
        expect(ME.fromText("x^2").toLatex()).toBe("x^{2}");
    });
});

// Small shared helpers used by the parser, evaluator, and UI.

import type { Complex, SafeResult } from "./types";

/** Run `fn`, capturing exceptions as a tagged result. */
export function safe<T>(fn: () => T): SafeResult<T> {
  try {
    return { ok: true, value: fn() };
  } catch (e) {
    return { ok: false, error: e instanceof Error ? e.message : String(e) };
  }
}

/** Format a real number compactly (integers verbatim, else 12 sig-figs). */
export function formatFloat(v: number): string {
  if (Number.isInteger(v)) return String(v);
  return String(Number(v.toPrecision(12)));
}

/** Format a `{ re, im }` value, collapsing negligible parts (a + b i / b i / a). */
export function formatComplex({ re, im }: Complex): string {
  const tol = 1e-9 * Math.max(1, Math.abs(re), Math.abs(im));
  const imZero = Math.abs(im) <= tol;
  const reZero = Math.abs(re) <= tol;
  if (imZero) return formatFloat(re);
  const mag = formatFloat(Math.abs(im));
  const imPart = mag === "1" ? "i" : `${mag} i`;
  if (reZero) return (im < 0 ? "−" : "") + imPart;
  return `${formatFloat(re)} ${im < 0 ? "−" : "+"} ${imPart}`;
}

/** Structural equality for `Tree` (and other JSON-ish) values. */
export function deepEqual(a: unknown, b: unknown): boolean {
  if (Array.isArray(a) && Array.isArray(b)) {
    return a.length === b.length && a.every((x, i) => deepEqual(x, b[i]));
  }
  return a === b;
}

/** Order-insensitive equality of two string lists (as multisets). */
export function sameStringSet(a: string[], b: string[]): boolean {
  if (a.length !== b.length) return false;
  const counts = new Map<string, number>();
  for (const x of a) counts.set(x, (counts.get(x) ?? 0) + 1);
  for (const x of b) {
    const n = counts.get(x);
    if (!n) return false;
    counts.set(x, n - 1);
  }
  return true;
}

/**
 * Whether two complex values agree within a relative tolerance. `null` values
 * agree only with each other. The scale uses both magnitudes so the test is
 * symmetric in its arguments. Used to badge numeric/complex step results.
 */
export function complexAgrees(
  a: Complex | null,
  b: Complex | null,
): boolean {
  if (a === null && b === null) return true;
  if (a === null || b === null) return false;
  const scale = Math.max(
    1,
    Math.abs(a.re),
    Math.abs(a.im),
    Math.abs(b.re),
    Math.abs(b.im),
  );
  return (
    Math.abs(a.re - b.re) <= 1e-7 * scale &&
    Math.abs(a.im - b.im) <= 1e-7 * scale
  );
}

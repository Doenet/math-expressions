// Renders an AST in the JS `Tree` JSON shape (e.g. ["+", ["^", "x", 2], 1]) as
// an indented, color-coded tree. Both engines emit this same shape.

function classify(head) {
  const ops = new Set([
    "+",
    "-",
    "*",
    "/",
    "^",
    "=",
    "<",
    ">",
    "le",
    "ge",
    "ne",
    "and",
    "or",
    "not",
    "union",
    "intersect",
  ]);
  if (ops.has(head)) return "node-op";
  if (head === "apply") return "node-apply";
  if (
    [
      "tuple",
      "list",
      "set",
      "array",
      "vector",
      "altvector",
      "interval",
      "matrix",
    ].includes(head)
  )
    return "node-seq";
  return "node-head";
}

function Node({ value }) {
  if (Array.isArray(value)) {
    const [head, ...children] = value;
    return (
      <li>
        <span className={classify(head)}>{String(head)}</span>
        <ul>
          {children.map((c, i) => (
            <Node key={i} value={c} />
          ))}
        </ul>
      </li>
    );
  }
  const kind =
    typeof value === "number"
      ? "leaf-num"
      : typeof value === "boolean"
        ? "leaf-bool"
        : "leaf-sym";
  return (
    <li>
      <span className={kind}>{String(value)}</span>
    </li>
  );
}

export default function Tree({ value }) {
  return (
    <ul className="ast-tree">
      <Node value={value} />
    </ul>
  );
}

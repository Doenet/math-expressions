import { useMemo } from "react";
import katex from "katex";

/** Render a LaTeX string to HTML, swallowing errors into a red fallback. */
export default function Katex({ tex, display = false }) {
  const html = useMemo(() => {
    try {
      return katex.renderToString(tex ?? "", {
        displayMode: display,
        throwOnError: false,
        errorColor: "#c00",
      });
    } catch {
      return null;
    }
  }, [tex, display]);

  if (html == null) return <code className="err">{tex}</code>;
  return <span dangerouslySetInnerHTML={{ __html: html }} />;
}

/*
 * Presentation MathML → LaTeX.
 *
 * Ported from the legacy `lib/converters/mml-to-latex.js`. This is pure
 * notation shuffling — an XML tree in, a LaTeX string out — so it stays in
 * TypeScript rather than crossing into the Rust core, which has no MathML
 * reader.
 *
 * The legacy converter delegated XML parsing to the `xml-parser` package. That
 * package is unmaintained CommonJS and is only present in this monorepo as a
 * transitive dependency of the published legacy library we diff against, so a
 * verbatim port of its 1.2.1 `parse()` lives below instead of being added as a
 * dependency. Its exact behaviour is load-bearing: it leaves text content
 * completely unescaped, which is what lets the entity table in this file see
 * raw `&sdot;` / `&#x2212;` strings rather than the characters they denote.
 */

// fix missing semicolons
const entities: Record<string, string> = {
  "&#913;": "\\Alpha",
  "&Alpha;": "\\Alpha",
  "&#x0391;": "\\Alpha",
  "\\u0391;": "\\Alpha",
  "&#914;": "\\Beta",
  "&Beta;": "\\Beta",
  "&#x0392;": "\\Beta",
  "\\u0392;": "\\Beta",
  "&#915;": "\\Gamma",
  "&Gamma;": "\\Gamma",
  "&#x0393;": "\\Gamma",
  "\\u0393;": "\\Gamma",
  "&#916;": "\\Delta",
  "&Delta;": "\\Delta",
  "&#x0394;": "\\Delta",
  "\\u0394;": "\\Delta",
  "&#917;": "\\Epsilon",
  "&Epsilon;": "\\Epsilon",
  "&#x0395;": "\\Epsilon",
  "\\u0395;": "\\Epsilon",
  "&#918;": "\\Zeta",
  "&Zeta;": "\\Zeta",
  "&#x0396;": "\\Zeta",
  "\\u0396;": "\\Zeta",
  "&#919;": "\\Eta",
  "&Eta;": "\\Eta",
  "&#x0397;": "\\Eta",
  "\\u0397;": "\\Eta",
  "&#920;": "\\Theta",
  "&Theta;": "\\Theta",
  "&#x0398;": "\\Theta",
  "\\u0398;": "\\Theta",
  "&#921;": "\\Iota",
  "&Iota;": "\\Iota",
  "&#x0399;": "\\Iota",
  "\\u0399;": "\\Iota",
  "&#922;": "\\Kappa",
  "&Kappa;": "\\Kappa",
  "&#x039A;": "\\Kappa",
  "\\u039A;": "\\Kappa",
  "&#923;": "\\Lambda",
  "&Lambda;": "\\Lambda",
  "&#x039B;": "\\Lambda",
  "\\u039B;": "\\Lambda",
  "&#924;": "\\Mu",
  "&Mu;": "\\Mu",
  "&#x039C;": "\\Mu",
  "\\u039C;": "\\Mu",
  "&#925;": "\\Nu",
  "&Nu;": "\\Nu",
  "&#x039D;": "\\Nu",
  "\\u039D;": "\\Nu",
  "&#926;": "\\Xi",
  "&Xi;": "\\Xi",
  "&#x039E;": "\\Xi",
  "\\u039E;": "\\Xi",
  "&#927;": "\\Omicron",
  "&Omicron;": "\\Omicron",
  "&#x039F;": "\\Omicron",
  "\\u039F;": "\\Omicron",
  "&#928;": "\\Pi",
  "&Pi;": "\\Pi",
  "&#x03A0;": "\\Pi",
  "\\u03A0;": "\\Pi",
  "&#929;": "\\Rho",
  "&Rho;": "\\Rho",
  "&#x03A1;": "\\Rho",
  "\\u03A1;": "\\Rho",
  "&#931;": "\\Sigma",
  "&Sigma;": "\\Sigma",
  "&#x03A3;": "\\Sigma",
  "\\u03A3;": "\\Sigma",
  "&#932;": "\\Tau",
  "&Tau;": "\\Tau",
  "&#x03A4;": "\\Tau",
  "\\u03A4;": "\\Tau",
  "&#933;": "\\Upsilon",
  "&Upsilon;": "\\Upsilon",
  "&#x03A5;": "\\Upsilon",
  "\\u03A5;": "\\Upsilon",
  "&#934;": "\\Phi",
  "&Phi;": "\\Phi",
  "&#x03A6;": "\\Phi",
  "\\u03A6;": "\\Phi",
  "&#935;": "\\Chi",
  "&Chi;": "\\Chi",
  "&#x03A7;": "\\Chi",
  "\\u03A7;": "\\Chi",
  "&#936;": "\\Psi",
  "&Psi;": "\\Psi",
  "&#x03A8;": "\\Psi",
  "\\u03A8;": "\\Psi",
  "&#937;": "\\Omega",
  "&Omega;": "\\Omega",
  "&#x03A9;": "\\Omega",
  "\\u03A9;": "\\Omega",
  "&#945;": "\\alpha",
  "&alpha;": "\\alpha",
  "&#x03B1;": "\\alpha",
  "\\u03B1;": "\\alpha",
  "&#946;": "\\beta",
  "&beta;": "\\beta",
  "&#x03B2;": "\\beta",
  "\\u03B2;": "\\beta",
  "&#947;": "\\gamma",
  "&gamma;": "\\gamma",
  "&#x03B3;": "\\gamma",
  "\\u03B3;": "\\gamma",
  "&#948;": "\\delta",
  "&delta;": "\\delta",
  "&#x03B4;": "\\delta",
  "\\u03B4;": "\\delta",
  "&#949;": "\\epsilon",
  "&epsilon;": "\\epsilon",
  "&#x03B5;": "\\epsilon",
  "\\u03B5;": "\\epsilon",
  "&#950;": "\\zeta",
  "&zeta;": "\\zeta",
  "&#x03B6;": "\\zeta",
  "\\u03B6;": "\\zeta",
  "&#951;": "\\eta",
  "&eta;": "\\eta",
  "&#x03B7;": "\\eta",
  "\\u03B7;": "\\eta",
  "&#952;": "\\theta",
  "&theta;": "\\theta",
  "&#x03B8;": "\\theta",
  "\\u03B8;": "\\theta",
  "&#953;": "\\iota",
  "&iota;": "\\iota",
  "&#x03B9;": "\\iota",
  "\\u03B9;": "\\iota",
  "&#954;": "\\kappa",
  "&kappa;": "\\kappa",
  "&#x03BA;": "\\kappa",
  "\\u03BA;": "\\kappa",
  "&#955;": "\\lambda",
  "&lambda;": "\\lambda",
  "&#x03BB;": "\\lambda",
  "\\u03BB;": "\\lambda",
  "&#956;": "\\mu",
  "&mu;": "\\mu",
  "&#x03BC;": "\\mu",
  "\\u03BC;": "\\mu",
  "&#957;": "\\nu",
  "&nu;": "\\nu",
  "&#x03BD;": "\\nu",
  "\\u03BD;": "\\nu",
  "&#958;": "\\xi",
  "&xi;": "\\xi",
  "&#x03BE;": "\\xi",
  "\\u03BE;": "\\xi",
  "&#959;": "\\omicron",
  "&omicron;": "\\omicron",
  "&#x03BF;": "\\omicron",
  "\\u03BF;": "\\omicron",
  "&#960;": "\\pi",
  "&pi;": "\\pi",
  "&#x03C0;": "\\pi",
  "\\u03C0;": "\\pi",
  "&#961;": "\\rho",
  "&rho;": "\\rho",
  "&#x03C1;": "\\rho",
  "\\u03C1;": "\\rho",
  "&#962;": "\\sigma",
  // Legacy carried a bare `";"` here — the remains of a `&sigmaf;` key that
  // lost its head — with a note calling it unreachable because "text content
  // never equals `;` alone after the surrounding markup is stripped". It does:
  // `content()` matches `/^([^<]*)/`, so `<mo>;</mo>` yields exactly `";"`, and
  // MathJax emits `<mo>;</mo>` for every semicolon separator. `f(x; y)`
  // converted to `f ( x \sigma y )`. Spelled as the entity it was meant to be.
  "&sigmaf;": "\\sigma",
  "&#x03C2;": "\\sigma",
  "\\u03C2;": "\\sigma",
  "&#963;": "\\sigma",
  "&sigma;": "\\sigma",
  "&#x03C3;": "\\sigma",
  "\\u03C3;": "\\sigma",
  "&#964;": "\\tau",
  "&tau;": "\\tau",
  "&#x03C4;": "\\tau",
  "\\u03C4;": "\\tau",
  "&#965;": "\\upsilon",
  "&upsilon;": "\\upsilon",
  "&#x03C5;": "\\upsilon",
  "\\u03C5;": "\\upsilon",
  "&#966;": "\\phi",
  "&phi;": "\\phi",
  "&#x03C6;": "\\phi",
  "\\u03C6;": "\\phi",
  "&#967;": "\\chi",
  "&chi;": "\\chi",
  "&#x03C7;": "\\chi",
  "\\u03C7;": "\\chi",
  "&#968;": "\\psi",
  "&psi;": "\\psi",
  "&#x03C8;": "\\psi",
  "\\u03C8;": "\\psi",
  "&#969;": "\\omega",
  "&omega;": "\\omega",
  "&#x03C9;": "\\omega",
  "\\u03C9;": "\\omega",
  "&#x2212;": "-",
  "&minus;": "-",
  "&#x221E;": "\\infty",
  "&#8734;": "\\infty",
  "&infin;": "\\infty",
  "&sdot;": "\\cdot",
  "&#x22C5;": "\\cdot",
  "&#8901;": "\\cdot",
  "&times;": "\\times",
  "&#x00D7;": "\\times",
  "&#215;": "\\times",
};

// ---------------------------------------------------------------------------
// XML parsing (verbatim port of `xml-parser` 1.2.1)
// ---------------------------------------------------------------------------

/** One element of the parsed document. */
export interface XmlNode {
  name: string;
  attributes: Record<string, string>;
  children: XmlNode[];
  /**
   * Text directly after the open tag only — this parser reads content once and
   * then switches to children, so text interleaved between child elements is
   * silently dropped. Absent entirely on self-closing tags.
   */
  content?: string;
}

export interface XmlDocument {
  declaration?: { attributes: Record<string, string> };
  root?: XmlNode;
}

function parseString(xml: string): XmlDocument {
  xml = xml.trim();

  // strip comments
  xml = xml.replace(/<!--[\s\S]*?-->/g, "");

  return document();

  /**
   * XML document.
   */
  function document(): XmlDocument {
    return {
      declaration: declaration(),
      root: tag(),
    };
  }

  /**
   * Declaration.
   */
  function declaration(): { attributes: Record<string, string> } | undefined {
    const m = match(/^<\?xml\s*/);
    if (!m) return;

    // tag
    const node: { attributes: Record<string, string> } = {
      attributes: {},
    };

    // attributes
    while (!(eos() || is("?>"))) {
      const attr = attribute();
      if (!attr) return node;
      node.attributes[attr.name] = attr.value;
    }

    match(/\?>\s*/);

    return node;
  }

  /**
   * Tag.
   */
  function tag(): XmlNode | undefined {
    const m = match(/^<([\w-:.]+)\s*/);
    if (!m) return;

    // name
    const node: XmlNode = {
      name: m[1],
      attributes: {},
      children: [],
    };

    // attributes
    while (!(eos() || is(">") || is("?>") || is("/>"))) {
      const attr = attribute();
      if (!attr) return node;
      node.attributes[attr.name] = attr.value;
    }

    // self closing tag
    if (match(/^\s*\/>\s*/)) {
      return node;
    }

    match(/\??>\s*/);

    // content
    node.content = content();

    // children
    let child: XmlNode | undefined;
    while ((child = tag())) {
      node.children.push(child);
    }

    // closing
    match(/^<\/[\w-:.]+>\s*/);

    return node;
  }

  /**
   * Text content.
   */
  function content(): string {
    const m = match(/^([^<]*)/);
    if (m) return m[1];
    return "";
  }

  /**
   * Attribute.
   */
  function attribute(): { name: string; value: string } | undefined {
    const m = match(/([\w:-]+)\s*=\s*("[^"]*"|'[^']*'|\w+)\s*/);
    if (!m) return;
    return { name: m[1], value: strip(m[2]) };
  }

  /**
   * Strip quotes from `val`.
   */
  function strip(val: string): string {
    return val.replace(/^['"]|['"]$/g, "");
  }

  /**
   * Match `re` and advance the string.
   *
   * Upstream quirk: several of the patterns above are unanchored, yet the
   * match length is always sliced off the *front* of the input. That only
   * works because every unanchored call site is already positioned on the
   * text it expects to consume.
   */
  function match(re: RegExp): RegExpMatchArray | undefined {
    const m = xml.match(re);
    if (!m) return;
    xml = xml.slice(m[0].length);
    return m;
  }

  /**
   * End-of-source.
   */
  function eos(): boolean {
    return 0 == xml.length;
  }

  /**
   * Check for `prefix`.
   */
  function is(prefix: string): boolean {
    return 0 == xml.indexOf(prefix);
  }
}

// ---------------------------------------------------------------------------
// MathML → LaTeX
// ---------------------------------------------------------------------------

class mmlToLatex {
  // This is an awfully weak MathML parser, but it's good enough for what MathJax generates
  parse(mml: XmlNode): string | undefined {
    // math identifier
    if (mml.name === "mi") {
      if (entities[mml.content]) {
        return entities[mml.content];
      }

      // Multi-letter identifiers are assumed to name a LaTeX macro (`sin`,
      // `log`, …); single letters are variables and pass through as-is.
      if (mml.content.length > 1) {
        return "\\" + mml.content;
      } else {
        return mml.content;
      }
    } else if (mml.name === "mn") {
      // math number
      return mml.content;
    } else if (mml.name === "msup") {
      // superscript
      return (
        this.parse(mml.children[0]) + "^{" + this.parse(mml.children[1]) + "}"
      );
    } else if (mml.name === "mroot") {
      // root
      //
      // Legacy bug preserved: the index and the radicand both read
      // `children[1]`, so `<mroot><mi>x</mi><mn>3</mn></mroot>` renders as
      // `\sqrt[3]{3}`. Nothing in the suite pins `mroot`, and "correcting" it
      // here would silently change output for any caller that already
      // compensates.
      return (
        "\\sqrt[" +
        this.parse(mml.children[1]) +
        "]{" +
        this.parse(mml.children[1]) +
        "}"
      );
    } else if (mml.name === "mfrac") {
      return (
        "\\frac{" +
        this.parse(mml.children[0]) +
        "}{" +
        this.parse(mml.children[1]) +
        "}"
      );
    } else if (mml.name === "msqrt") {
      // square root
      return "\\sqrt{" + mml.children.map((v) => this.parse(v)).join(" ") + "}";
    } else if (mml.name === "mo") {
      // math operator
      if (entities[mml.content]) {
        return entities[mml.content];
      } else if (mml.content === "&#x2061;") {
        // U+2061 FUNCTION APPLICATION: MathJax's invisible operator, which has
        // no LaTeX spelling — a space is the closest equivalent.
        return " ";
      } else {
        return mml.content;
      }
    } else if (
      mml.name === "mrow" &&
      mml.attributes.class === "MJX-TeXAtom-ORD"
    ) {
      // MathJax's "ordinary atom" wrapper carries no grouping of its own, so
      // unlike a plain `mrow` it must not introduce parentheses.
      return mml.children.map((v) => this.parse(v)).join(" ");
    } else if (mml.name === "math" || mml.name === "mrow") {
      return "(" + mml.children.map((v) => this.parse(v)).join(" ") + ")";
    }

    // Unrecognized element: legacy fell off the end of the chain and returned
    // undefined, which the callers happily stringify into "undefined".
    return undefined;
  }

  convert(xml: string): string | undefined {
    return this.parse(parseString(xml).root);
  }
}

export default mmlToLatex;

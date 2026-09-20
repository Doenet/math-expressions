// The original `ParseError` type. The Rust parser throws its own message string
// through wasm-bindgen; this class exists so specs importing it resolve, and so
// `new ParseError(...)` works if anything constructs one directly.
export class ParseError extends Error {
  /** Where in the source the parse failed, when the thrower knows. */
  location: unknown;
  constructor(message: string, location?: unknown) {
    super(message);
    this.name = "ParseError";
    this.location = location;
  }
}

export default ParseError;
